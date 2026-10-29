package io.github.norbertque.pixeltone.playback

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import androidx.annotation.OptIn
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.DefaultRenderersFactory
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.audio.AudioSink
import androidx.media3.session.CacheBitmapLoader
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import com.google.common.util.concurrent.SettableFuture
import io.github.norbertque.pixeltone.MainActivity
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.library.PlayCounter
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.library.mayReadAudio
import io.github.norbertque.pixeltone.library.trackUri
import io.github.norbertque.pixeltone.remote.HandOverItem
import io.github.norbertque.pixeltone.remote.PairedPc
import io.github.norbertque.pixeltone.remote.PcCommand
import io.github.norbertque.pixeltone.remote.PcPlayer
import io.github.norbertque.pixeltone.remote.PhoneFileServer
import io.github.norbertque.pixeltone.remote.RemoteHost
import io.github.norbertque.pixeltone.remote.RemoteProtocol
import io.github.norbertque.pixeltone.remote.RemoteSession
import io.github.norbertque.pixeltone.usb.UsbAudioSink
import io.github.norbertque.pixeltone.usb.UsbDeviceVolumePlayer
import io.github.norbertque.pixeltone.usb.UsbExclusiveState
import io.github.norbertque.pixeltone.usbaudio.UsbAudioSession
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Plays the queue in the background. Media3 shows the media notification and the lock screen
 * controls for it, takes the headset and Bluetooth buttons, and keeps the service in the
 * foreground while it plays.
 *
 * While playback is on a Pixeltone PC, the session runs [PcPlayer] instead of the phone's own
 * player, and the phone serves the handed-over tracks to the PC.
 *
 * With exclusive USB output on and a DAC ready, the phone's player is rebuilt to play into the
 * DAC directly ([UsbAudioSink], bit-perfect) and back to Android's output when the DAC goes;
 * the queue and position move with it.
 */
@OptIn(UnstableApi::class)
class PlaybackService : MediaSessionService() {
    private val scope = MainScope()
    // Outlives the service, so the last save still reaches the disk; one write at a time.
    private val writes = CoroutineScope(SupervisorJob() + Dispatchers.IO.limitedParallelism(1))
    private val app get() = application as PixeltoneApp
    private lateinit var store: QueueStore
    private lateinit var local: ExoPlayer
    private lateinit var pc: PcPlayer

    /** The DAC the phone's player plays into; null for Android's own output. */
    private var usbInUse: UsbAudioSession? = null

    /** Playback stopped because the DAC went away: it goes on when the DAC is back. */
    private var pausedByUnplug = false
    private val saver = object : Player.Listener {
        override fun onEvents(player: Player, events: Player.Events) = saveSoon()
    }

    /** The phone's own listens go into the play history. */
    private val counter by lazy { PlayCounter(System::currentTimeMillis, app.history::record) }
    private val counting = object : Player.Listener {
        override fun onMediaItemTransition(mediaItem: MediaItem?, reason: Int) {
            counter.onTrack(trackOf(mediaItem), local.isPlaying, again = reason == Player.MEDIA_ITEM_TRANSITION_REASON_REPEAT)
        }

        override fun onIsPlayingChanged(isPlaying: Boolean) = counter.onPlaying(isPlaying)

        // A jump back to the start (previous, a seek) is a new listen, as on Windows.
        override fun onPositionDiscontinuity(old: Player.PositionInfo, new: Player.PositionInfo, reason: Int) {
            val restarted = reason == Player.DISCONTINUITY_REASON_SEEK && old.mediaItemIndex == new.mediaItemIndex &&
                new.positionMs < 1_500 && old.positionMs > 10_000
            if (restarted) counter.onTrack(trackOf(new.mediaItem), local.isPlaying, again = true)
        }
    }

    private fun trackOf(item: MediaItem?): Track? =
        item?.mediaId?.toLongOrNull()?.let { app.library.index.value?.track(it) }
    private var session: MediaSession? = null
    private var pendingSave: Job? = null

    /** The phone's queue, waiting for the PC to be connected to go over. */
    private var pendingHandOver: HandOver? = null

    private class HandOver(val items: List<MediaItem>, val start: Int, val positionMs: Long)

    override fun onCreate() {
        super.onCreate()
        store = QueueStore(File(filesDir, "queue.json"))
        local = buildPlayer(usb = null)
        pc = PcPlayer(mainLooper, app.remote, app.library) { items, start, positionMs ->
            app.remote.connection.session.value?.let { handOver(it.pc, HandOver(items, start, positionMs)) }
        }
        val openApp = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        session = MediaSession.Builder(this, local)
            .setSessionActivity(openApp)
            .setBitmapLoader(CacheBitmapLoader(AlbumArtBitmapLoader(this, app.remote)))
            .setCallback(SessionCallback())
            .build()
        app.remote.host = object : RemoteHost {
            override fun playOn(pc: PairedPc) = moveTo(pc)

            override fun playHere() = takeBack()
        }
        scope.launch { app.remote.connection.session.collect(::follow) }
        scope.launch { app.usb.sessionLost.collect { lostDac() } }
        scope.launch { app.usb.state.collect(::followUsb) }
        scope.launch { app.sleep.state.collectLatest(::sleep) }
        pc.addListener(stopAfterPcTrack)
        scope.launch { restore() }
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? = session

    override fun onTaskRemoved(rootIntent: Intent?) {
        save()
        super.onTaskRemoved(rootIntent)
    }

    override fun onDestroy() {
        save()
        counter.flush()
        app.remote.host = null
        app.remote.connection.end()
        app.remote.files.stop()
        session?.release()
        session = null
        pc.release()
        local.release()
        scope.cancel()
        super.onDestroy()
    }

    /* ------------------------------- Sleep timer ------------------------------- */

    private suspend fun sleep(sleep: Sleep) {
        local.pauseAtEndOfMediaItems = sleep is Sleep.EndOfTrack
        if (sleep is Sleep.At) {
            delay((sleep.endsAt - System.currentTimeMillis()).coerceAtLeast(0))
            session?.player?.pause()
            app.sleep.cancel()
        }
    }

    /** At the end of the track, the phone's player stops by itself ([ExoPlayer.setPauseAtEndOfMediaItems]). */
    private val stopAfterLocalTrack = object : Player.Listener {
        override fun onPlayWhenReadyChanged(playWhenReady: Boolean, reason: Int) {
            if (!playWhenReady && reason == Player.PLAY_WHEN_READY_CHANGE_REASON_END_OF_MEDIA_ITEM && app.sleep.state.value == Sleep.EndOfTrack) {
                app.sleep.cancel()
            }
        }
    }

    /** A PC goes on to its next track: pause it there. */
    private val stopAfterPcTrack = object : Player.Listener {
        override fun onMediaItemTransition(mediaItem: MediaItem?, reason: Int) {
            if (reason == Player.MEDIA_ITEM_TRANSITION_REASON_AUTO && app.sleep.state.value == Sleep.EndOfTrack && session?.player === pc) {
                pc.pause()
                app.sleep.cancel()
            }
        }
    }

    /* ------------------------------ Phone output ------------------------------ */

    /** The phone's player, into Android's output or straight into a USB DAC. */
    private fun buildPlayer(usb: UsbAudioSession?): ExoPlayer {
        val renderers = object : DefaultRenderersFactory(this) {
            override fun buildAudioSink(
                context: Context,
                enableFloatOutput: Boolean,
                enableAudioOutputPlaybackParams: Boolean,
            ): AudioSink? =
                if (usb != null) {
                    // It takes float directly, so decoders hand over 24-bit sources whole.
                    UsbAudioSink(usb, app.usb::onSinkFormatChanged, app.usb::onSessionDead)
                } else {
                    super.buildAudioSink(context, enableFloatOutput, enableAudioOutputPlaybackParams)
                }
        }
        return ExoPlayer.Builder(this, renderers)
            .setAudioAttributes(
                AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(),
                true, // handle audio focus: pause for calls and other players (never ducks on a DAC)
            )
            // Pause when headphones come out; a DAC played into directly isn't one of Android's
            // outputs (its unplugging pauses through the USB controller instead).
            .setHandleAudioBecomingNoisy(usb == null)
            .setWakeMode(C.WAKE_MODE_LOCAL)
            .build()
            .also {
                it.addListener(saver)
                it.addListener(counting)
                it.addListener(stopAfterLocalTrack)
                it.pauseAtEndOfMediaItems = app.sleep.state.value == Sleep.EndOfTrack
            }
    }

    /** The phone's player as the session shows it: on a DAC, the volume keys turn the DAC's volume. */
    private fun localForSession(): Player = if (usbInUse != null) UsbDeviceVolumePlayer(local, app.usb) else local

    private fun followUsb(state: UsbExclusiveState) {
        val ready = state is UsbExclusiveState.Ready || state is UsbExclusiveState.Active
        val wanted = app.usb.activeSession.takeIf { ready }
        if (wanted === usbInUse) return
        if (usbInUse?.isAlive == false) lostDac()
        switchOutput(wanted)
        if (wanted != null && pausedByUnplug) {
            pausedByUnplug = false
            val device = (state as? UsbExclusiveState.Ready)?.device ?: (state as? UsbExclusiveState.Active)?.device
            if (device != null && app.usbSettings.rememberedDevices.value[device.key]?.autoResume != false) local.play()
        }
    }

    /** The DAC was unplugged (or stopped answering) while the phone played into it. */
    private fun lostDac() {
        if (usbInUse == null) return
        if (local.playWhenReady) {
            pausedByUnplug = true
            local.pause()
        }
    }

    /** A new phone player for the output, taking over the queue, position and modes. */
    private fun switchOutput(usb: UsbAudioSession?) {
        val old = local
        val next = buildPlayer(usb)
        if (old.mediaItemCount > 0) {
            next.setMediaItems(List(old.mediaItemCount) { old.getMediaItemAt(it) }, old.currentMediaItemIndex, old.currentPosition)
        }
        next.repeatMode = old.repeatMode
        next.shuffleModeEnabled = old.shuffleModeEnabled
        next.prepare()
        next.playWhenReady = old.playWhenReady
        local = next
        usbInUse = usb
        session?.let { if (it.player !== pc) it.player = localForSession() }
        old.removeListener(saver)
        old.removeListener(counting)
        old.removeListener(stopAfterLocalTrack)
        old.release()
    }

    /* ----------------------------- Phone and PC ----------------------------- */

    /** The session runs whichever player has the playback: the PC's while a session with one is on. */
    private fun follow(remote: RemoteSession?) {
        val session = session ?: return
        if (remote != null && session.player !== pc) {
            local.pause()
            session.player = pc
        } else if (remote == null && session.player === pc) {
            session.player = localForSession()
            app.remote.files.stop()
            pendingHandOver = null
        }
        if (remote?.connected == true) {
            pendingHandOver?.let { if (handOver(remote.pc, it)) pendingHandOver = null }
        }
    }

    /** Moves playback to [target]: the phone's queue and position go over, and the phone pauses. */
    private fun moveTo(target: PairedPc) {
        val connection = app.remote.connection
        val current = connection.session.value
        pendingHandOver = when {
            // From one PC to another: whatever played goes on there, if it was the phone's.
            current != null -> {
                val ids = pc.rows.mapNotNull { app.remote.files.trackIdIn(it.path) }
                val index = app.library.index.value
                val tracks = ids.mapNotNull { index?.track(it) }
                val playing = current.state?.track?.path?.let(app.remote.files::trackIdIn)
                if (tracks.isEmpty()) {
                    null
                } else {
                    connection.send(PcCommand.simple("pause"))
                    HandOver(tracks.map { it.toMediaItem() }, tracks.indexOfFirst { it.id == playing }.coerceAtLeast(0), connection.positionNow())
                }
            }
            local.mediaItemCount > 0 -> HandOver(
                List(local.mediaItemCount) { local.getMediaItemAt(it) },
                local.currentMediaItemIndex,
                local.currentPosition,
            )
            else -> null
        }
        local.pause()
        app.remote.files.start()
        connection.begin(target)
    }

    /** Sends a queue of phone tracks to the PC; false when it can't go yet. */
    private fun handOver(target: PairedPc, request: HandOver): Boolean {
        val address = PhoneFileServer.addressTowards(target.host, target.port) ?: return false
        app.remote.files.start()
        val index = app.library.index.value
        // The PC takes so many tracks at most: a window around the one to play.
        val from = (request.start - RemoteProtocol.MAX_HAND_OVER / 2).coerceIn(0, maxOf(0, request.items.size - RemoteProtocol.MAX_HAND_OVER))
        val window = request.items.drop(from).take(RemoteProtocol.MAX_HAND_OVER)
        val items = window.mapNotNull { item ->
            val id = item.mediaId.toLongOrNull() ?: return@mapNotNull null
            val track = index?.track(id)
            HandOverItem(
                url = app.remote.files.trackUrl(address, id),
                artUrl = track?.let { app.remote.files.coverUrl(address, it.albumId) },
                title = track?.title ?: item.mediaMetadata.title?.toString(),
                artist = track?.artist ?: item.mediaMetadata.artist?.toString(),
                album = track?.album ?: item.mediaMetadata.albumTitle?.toString(),
                durationMs = track?.durationMs,
            )
        }
        if (items.isEmpty()) return true
        val repeat = when (local.repeatMode) {
            Player.REPEAT_MODE_ALL -> "all"
            Player.REPEAT_MODE_ONE -> "one"
            else -> "off"
        }
        return app.remote.connection.send(
            PcCommand.load(items, (request.start - from).coerceIn(0, items.lastIndex), request.positionMs, play = true, repeat = repeat),
        )
    }

    /**
     * Brings playback back to the phone and pauses the PC. If the PC was playing a phone track,
     * the phone goes on with the PC's queue of phone tracks from there; else the phone's player
     * is as it was left.
     */
    private fun takeBack() {
        val connection = app.remote.connection
        val remote = connection.session.value ?: return
        val playing = remote.state?.track?.path?.let(app.remote.files::trackIdIn)
        val position = connection.positionNow()
        val index = app.library.index.value
        val tracks = pc.rows.mapNotNull { row -> app.remote.files.trackIdIn(row.path)?.let { index?.track(it) } }
        val wasPlaying = remote.state?.isPlaying == true
        connection.send(PcCommand.simple("pause"))
        connection.end()
        if (playing != null && tracks.isNotEmpty()) {
            local.setMediaItems(tracks.map { it.toMediaItem() }, tracks.indexOfFirst { it.id == playing }.coerceAtLeast(0), position)
            local.prepare()
            if (wasPlaying) local.play()
        }
    }

    /* ------------------------------ Saved queue ------------------------------ */

    private fun saveSoon() {
        pendingSave?.cancel()
        pendingSave = scope.launch {
            delay(1_000)
            save()
        }
    }

    private fun save() {
        pendingSave?.cancel()
        if (!::local.isInitialized) return
        val queue = SavedQueue(
            trackIds = List(local.mediaItemCount) { local.getMediaItemAt(it).mediaId.toLongOrNull() ?: -1L },
            index = local.currentMediaItemIndex,
            positionMs = local.currentPosition,
            repeatMode = local.repeatMode,
            shuffle = local.shuffleModeEnabled,
        )
        writes.launch { store.save(queue) }
    }

    /** Puts the saved queue back, paused, unless something is playing already. */
    private suspend fun restore() {
        if (local.mediaItemCount > 0) return
        val saved = withContext(Dispatchers.IO) { store.load() } ?: return
        val resumed = resume(saved) ?: return
        // The player may have been rebuilt meanwhile (a DAC came up): the one in use now.
        val player = local
        if (player.mediaItemCount > 0) return
        player.setMediaItems(resumed.mediaItems, resumed.startIndex, resumed.startPositionMs)
        player.repeatMode = saved.repeatMode
        player.shuffleModeEnabled = saved.shuffle
        player.prepare()
    }

    /** The saved queue as playable items, without the tracks that left the phone since. */
    private suspend fun resume(saved: SavedQueue): MediaSession.MediaItemsWithStartPosition? {
        if (!mayReadAudio()) return null
        val library = app.library
        library.start()
        val index = library.index.filterNotNull().first()
        val kept = saved.trackIds.withIndex().mapNotNull { (i, id) -> index.track(id)?.let { i to it } }
        if (kept.isEmpty()) return null
        val start = kept.indexOfFirst { it.first == saved.index }
        return MediaSession.MediaItemsWithStartPosition(
            kept.map { it.second.toMediaItem() },
            start.coerceAtLeast(0),
            if (start >= 0) saved.positionMs else 0,
        )
    }

    private inner class SessionCallback : MediaSession.Callback {
        // Items from a controller arrive without their file address: it follows from the track id.
        override fun onAddMediaItems(
            mediaSession: MediaSession,
            controller: MediaSession.ControllerInfo,
            mediaItems: List<MediaItem>,
        ): ListenableFuture<List<MediaItem>> =
            Futures.immediateFuture(
                mediaItems.mapNotNull { item ->
                    if (item.localConfiguration != null) {
                        item
                    } else {
                        item.mediaId.toLongOrNull()?.let { item.buildUpon().setUri(trackUri(it)).build() }
                    }
                },
            )

        // Play pressed on headphones or in the system's media controls after the app was closed.
        override fun onPlaybackResumption(
            mediaSession: MediaSession,
            controller: MediaSession.ControllerInfo,
            isForPlayback: Boolean,
        ): ListenableFuture<MediaSession.MediaItemsWithStartPosition> {
            val result = SettableFuture.create<MediaSession.MediaItemsWithStartPosition>()
            scope.launch {
                val saved = withContext(Dispatchers.IO) { store.load() }
                val resumed = saved?.let { resume(it) }
                if (resumed != null) result.set(resumed) else result.setException(IllegalStateException("No saved queue"))
            }
            return result
        }
    }
}
