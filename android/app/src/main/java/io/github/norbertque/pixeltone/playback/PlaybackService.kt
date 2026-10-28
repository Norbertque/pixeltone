package io.github.norbertque.pixeltone.playback

import android.app.PendingIntent
import android.content.Intent
import androidx.annotation.OptIn
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.CacheBitmapLoader
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import com.google.common.util.concurrent.SettableFuture
import io.github.norbertque.pixeltone.MainActivity
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.library.mayReadAudio
import io.github.norbertque.pixeltone.library.trackUri
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Plays the queue in the background. Media3 shows the media notification and the lock screen
 * controls for it, takes the headset and Bluetooth buttons, and keeps the service in the
 * foreground while it plays.
 */
@OptIn(UnstableApi::class)
class PlaybackService : MediaSessionService() {
    private val scope = MainScope()
    // Outlives the service, so the last save still reaches the disk; one write at a time.
    private val writes = CoroutineScope(SupervisorJob() + Dispatchers.IO.limitedParallelism(1))
    private lateinit var store: QueueStore
    private var session: MediaSession? = null
    private var pendingSave: Job? = null

    override fun onCreate() {
        super.onCreate()
        store = QueueStore(File(filesDir, "queue.json"))
        val player = ExoPlayer.Builder(this)
            .setAudioAttributes(
                AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(),
                true, // handle audio focus: pause for calls and other players
            )
            .setHandleAudioBecomingNoisy(true)
            .setWakeMode(C.WAKE_MODE_LOCAL)
            .build()
        player.addListener(
            object : Player.Listener {
                override fun onEvents(player: Player, events: Player.Events) = saveSoon()
            },
        )
        val openApp = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        session = MediaSession.Builder(this, player)
            .setSessionActivity(openApp)
            .setBitmapLoader(CacheBitmapLoader(AlbumArtBitmapLoader(this)))
            .setCallback(SessionCallback())
            .build()
        scope.launch { restore(player) }
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? = session

    override fun onTaskRemoved(rootIntent: Intent?) {
        save()
        super.onTaskRemoved(rootIntent)
    }

    override fun onDestroy() {
        save()
        session?.run {
            player.release()
            release()
        }
        session = null
        scope.cancel()
        super.onDestroy()
    }

    private fun saveSoon() {
        pendingSave?.cancel()
        pendingSave = scope.launch {
            delay(1_000)
            save()
        }
    }

    private fun save() {
        pendingSave?.cancel()
        val player = session?.player ?: return
        val queue = SavedQueue(
            trackIds = List(player.mediaItemCount) { player.getMediaItemAt(it).mediaId.toLongOrNull() ?: -1L },
            index = player.currentMediaItemIndex,
            positionMs = player.currentPosition,
            repeatMode = player.repeatMode,
            shuffle = player.shuffleModeEnabled,
        )
        writes.launch { store.save(queue) }
    }

    /** Puts the saved queue back, paused, unless something is playing already. */
    private suspend fun restore(player: ExoPlayer) {
        if (player.mediaItemCount > 0) return
        val saved = withContext(Dispatchers.IO) { store.load() } ?: return
        val resumed = resume(saved) ?: return
        if (player.mediaItemCount > 0) return
        player.setMediaItems(resumed.mediaItems, resumed.startIndex, resumed.startPositionMs)
        player.repeatMode = saved.repeatMode
        player.shuffleModeEnabled = saved.shuffle
        player.prepare()
    }

    /** The saved queue as playable items, without the tracks that left the phone since. */
    private suspend fun resume(saved: SavedQueue): MediaSession.MediaItemsWithStartPosition? {
        if (!mayReadAudio()) return null
        val library = (application as PixeltoneApp).library
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
