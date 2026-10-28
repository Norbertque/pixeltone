package io.github.norbertque.pixeltone.remote

import android.os.Looper
import androidx.annotation.OptIn
import androidx.media3.common.C
import androidx.media3.common.DeviceInfo
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.common.util.UnstableApi
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import io.github.norbertque.pixeltone.library.MediaStoreLibrary
import io.github.norbertque.pixeltone.library.albumArtUri
import kotlin.math.roundToInt
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

/**
 * The PC's player, as a Media3 player: while playback is on a PC, the media session runs this
 * instead of the phone's own player, so the app, the notification, the lock screen, headset
 * buttons and the volume keys all show and drive the PC.
 *
 * A list played while it's in charge goes to the PC as a hand-over ([onHandOver]).
 */
@OptIn(UnstableApi::class)
class PcPlayer(
    looper: Looper,
    private val remote: RemoteControl,
    private val library: MediaStoreLibrary,
    private val onHandOver: (items: List<MediaItem>, start: Int, positionMs: Long) -> Unit,
) : SimpleBasePlayer(looper) {
    private val scope = MainScope()
    private var session: RemoteSession? = null
    private var fetchedVersion: Long? = null
    private var fetchedFor: String? = null

    /** The PC's queue as last fetched. */
    var rows: List<PcQueueRow> = emptyList()
        private set

    init {
        scope.launch {
            remote.connection.session.collect { current ->
                session = current
                val state = current?.state
                if (current == null) {
                    rows = emptyList()
                    fetchedVersion = null
                    fetchedFor = null
                } else if (state != null && (state.queueVersion != fetchedVersion || current.pc.id != fetchedFor)) {
                    fetchedVersion = state.queueVersion
                    fetchedFor = current.pc.id
                    fetchQueue(current.pc, state.queueVersion)
                }
                invalidateState()
            }
        }
    }

    private fun fetchQueue(pc: PairedPc, version: Long) {
        scope.launch {
            val all = ArrayList<PcQueueRow>()
            runCatching {
                while (true) {
                    val page = remote.connection.queue(pc, all.size, QUEUE_PAGE)
                    all += page
                    if (page.size < QUEUE_PAGE) break
                }
            }.onFailure { return@launch }
            if (fetchedVersion == version && fetchedFor == pc.id) {
                rows = all
                invalidateState()
            }
        }
    }

    override fun getState(): State {
        val current = session
        val state = current?.state
        val playlist = rows.map { row ->
            MediaItemData.Builder(row.id)
                .setMediaItem(item(row))
                .setDurationUs(row.durationMs?.let { (it * 1000).toLong() } ?: C.TIME_UNSET)
                .setIsSeekable(true)
                .build()
        }
        val commands = Player.Commands.Builder().addAll(
            COMMAND_PLAY_PAUSE,
            COMMAND_STOP,
            COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM,
            COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
            COMMAND_SEEK_TO_PREVIOUS,
            COMMAND_SEEK_TO_NEXT_MEDIA_ITEM,
            COMMAND_SEEK_TO_NEXT,
            COMMAND_SEEK_TO_MEDIA_ITEM,
            COMMAND_SET_REPEAT_MODE,
            COMMAND_SET_SHUFFLE_MODE,
            COMMAND_GET_CURRENT_MEDIA_ITEM,
            COMMAND_GET_TIMELINE,
            COMMAND_GET_METADATA,
            COMMAND_SET_MEDIA_ITEM,
            COMMAND_CHANGE_MEDIA_ITEMS,
        )
        val builder = State.Builder()
            .setPlaylist(playlist)
            .setPlayWhenReady(state?.isPlaying == true, PLAY_WHEN_READY_CHANGE_REASON_REMOTE)
            .setPlaybackState(
                when {
                    playlist.isEmpty() -> STATE_IDLE
                    current?.connected != true -> STATE_BUFFERING
                    else -> STATE_READY
                },
            )
            .setCurrentMediaItemIndex(
                if (playlist.isEmpty()) C.INDEX_UNSET else (state?.queueIndex ?: 0).coerceIn(0, playlist.lastIndex),
            )
            .setContentPositionMs { remote.connection.positionNow() }
            .setRepeatMode(
                when (state?.repeat) {
                    "all" -> REPEAT_MODE_ALL
                    "one" -> REPEAT_MODE_ONE
                    else -> REPEAT_MODE_OFF
                },
            )
            .setShuffleModeEnabled(state?.shuffle == true)
        // The PC's volume, in steps for the phone's volume keys.
        val volume = current?.volume?.takeIf { it.adjustable }
        if (volume != null) {
            val steps = ((volume.max - volume.min) / VOLUME_STEP_DB).roundToInt().coerceAtLeast(1)
            commands.addAll(COMMAND_GET_DEVICE_VOLUME, COMMAND_SET_DEVICE_VOLUME_WITH_FLAGS, COMMAND_ADJUST_DEVICE_VOLUME_WITH_FLAGS)
            builder
                .setDeviceInfo(DeviceInfo.Builder(DeviceInfo.PLAYBACK_TYPE_REMOTE).setMinVolume(0).setMaxVolume(steps).build())
                .setDeviceVolume(((volume.db - volume.min) / VOLUME_STEP_DB).roundToInt().coerceIn(0, steps))
                .setIsDeviceMuted(volume.muted)
        } else {
            builder.setDeviceInfo(DeviceInfo.Builder(DeviceInfo.PLAYBACK_TYPE_REMOTE).build())
        }
        return builder.setAvailableCommands(commands.build()).build()
    }

    /** A phone track shows as the phone knows it; anything else as the PC describes it. */
    private fun item(row: PcQueueRow): MediaItem {
        val phoneTrack = remote.files.trackIdIn(row.path)?.let { library.index.value?.track(it) }
        val artwork = phoneTrack?.let { albumArtUri(it.albumId) } ?: row.coverKey?.let(::pcCoverUri)
        return MediaItem.Builder()
            .setMediaId(phoneTrack?.id?.toString() ?: "pc:${row.id}")
            .setMediaMetadata(
                MediaMetadata.Builder()
                    .setTitle(phoneTrack?.title ?: row.title)
                    .setArtist(phoneTrack?.artist ?: row.artist)
                    .setAlbumTitle(phoneTrack?.album ?: row.album)
                    .setArtworkUri(artwork)
                    .setIsPlayable(true)
                    .setIsBrowsable(false)
                    .build(),
            )
            .build()
    }

    private fun send(command: String): ListenableFuture<*> {
        remote.connection.send(command)
        return Futures.immediateVoidFuture()
    }

    override fun handleSetPlayWhenReady(playWhenReady: Boolean) = send(PcCommand.simple(if (playWhenReady) "play" else "pause"))

    override fun handleStop() = send(PcCommand.simple("pause"))

    override fun handleSeek(mediaItemIndex: Int, positionMs: Long, seekCommand: Int): ListenableFuture<*> =
        when (seekCommand) {
            COMMAND_SEEK_TO_NEXT, COMMAND_SEEK_TO_NEXT_MEDIA_ITEM -> send(PcCommand.simple("next"))
            COMMAND_SEEK_TO_PREVIOUS, COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM -> send(PcCommand.simple("previous"))
            COMMAND_SEEK_TO_MEDIA_ITEM, COMMAND_SEEK_TO_DEFAULT_POSITION -> {
                rows.getOrNull(mediaItemIndex)?.let { remote.connection.send(PcCommand.jump(it.id)) }
                if (positionMs > 0) send(PcCommand.seek(positionMs)) else Futures.immediateVoidFuture()
            }
            else -> {
                remote.connection.movedTo(positionMs)
                send(PcCommand.seek(positionMs))
            }
        }

    override fun handleSetRepeatMode(repeatMode: Int) = send(
        PcCommand.setRepeat(
            when (repeatMode) {
                REPEAT_MODE_ALL -> "all"
                REPEAT_MODE_ONE -> "one"
                else -> "off"
            },
        ),
    )

    override fun handleSetShuffleModeEnabled(shuffleModeEnabled: Boolean) = send(PcCommand.setShuffle(shuffleModeEnabled))

    override fun handleSetMediaItems(mediaItems: List<MediaItem>, startIndex: Int, startPositionMs: Long): ListenableFuture<*> {
        onHandOver(mediaItems, startIndex.coerceAtLeast(0), startPositionMs.takeIf { it != C.TIME_UNSET } ?: 0L)
        return Futures.immediateVoidFuture()
    }

    // The protocol has no queue edits: they leave the PC's queue as it is.
    override fun handleAddMediaItems(index: Int, mediaItems: List<MediaItem>) = Futures.immediateVoidFuture()

    override fun handleMoveMediaItems(fromIndex: Int, toIndex: Int, newIndex: Int) = Futures.immediateVoidFuture()

    override fun handleRemoveMediaItems(fromIndex: Int, toIndex: Int) = Futures.immediateVoidFuture()

    override fun handleReplaceMediaItems(fromIndex: Int, toIndex: Int, mediaItems: List<MediaItem>) = Futures.immediateVoidFuture()

    override fun handleSetDeviceVolume(deviceVolume: Int, flags: Int): ListenableFuture<*> {
        val volume = session?.volume ?: return Futures.immediateVoidFuture()
        remote.connection.setVolume((volume.min + deviceVolume * VOLUME_STEP_DB).coerceIn(volume.min, volume.max))
        return Futures.immediateVoidFuture()
    }

    override fun handleIncreaseDeviceVolume(flags: Int): ListenableFuture<*> = stepVolume(+1)

    override fun handleDecreaseDeviceVolume(flags: Int): ListenableFuture<*> = stepVolume(-1)

    private fun stepVolume(direction: Int): ListenableFuture<*> {
        val volume = session?.volume ?: return Futures.immediateVoidFuture()
        remote.connection.setVolume((volume.db + direction * VOLUME_STEP_DB).coerceIn(volume.min, volume.max))
        return Futures.immediateVoidFuture()
    }

    override fun handleRelease(): ListenableFuture<*> {
        scope.cancel()
        return Futures.immediateVoidFuture()
    }

    private companion object {
        const val QUEUE_PAGE = 500
        /** One press of a volume key: 2 dB on the PC. */
        const val VOLUME_STEP_DB = 2f
    }
}
