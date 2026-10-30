package io.github.norbertque.pixeltone.playback

import android.content.ComponentName
import android.content.Context
import androidx.annotation.MainThread
import androidx.core.content.ContextCompat
import androidx.media3.common.C
import androidx.media3.common.Player
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import io.github.norbertque.pixeltone.library.Track
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.flow

/** The app's side of [PlaybackService]: its state as flows, and the commands to send it. */
class PlayerConnection(private val context: Context) : PlayerControls {
    private var controller: MediaController? = null
    private var connecting = false
    private val waiting = ArrayList<MediaController.() -> Unit>()
    private val current = MutableStateFlow(PlaybackState())

    val state: StateFlow<PlaybackState> = current.asStateFlow()

    /** Where in the playing track it is, read four times a second while someone watches. */
    val positionMs: Flow<Long> = flow {
        while (true) {
            emit(controller?.currentPosition ?: 0L)
            delay(250)
        }
    }

    /** Connects to the playback service, starting it; commands sent meanwhile wait for it. */
    @MainThread
    fun connect() {
        if (controller != null || connecting) return
        connecting = true
        val token = SessionToken(context, ComponentName(context, PlaybackService::class.java))
        val future = MediaController.Builder(context, token).buildAsync()
        future.addListener(
            {
                connecting = false
                val connected = runCatching { future.get() }.getOrNull() ?: return@addListener
                controller = connected
                connected.addListener(listener)
                refresh(queueChanged = true)
                waiting.forEach { connected.it() }
                waiting.clear()
            },
            ContextCompat.getMainExecutor(context),
        )
    }

    /** Replaces the queue with [tracks] and plays from [start]. */
    fun play(tracks: List<Track>, start: Int) = command {
        setMediaItems(tracks.map(Track::toMediaItem), start, 0)
        shuffleModeEnabled = false
        prepare()
        play()
    }

    /** [tracks] right after the one that plays (or as the queue, when it's empty). */
    fun playNext(tracks: List<Track>) = command {
        if (mediaItemCount == 0) {
            setMediaItems(tracks.map(Track::toMediaItem))
            prepare()
        } else {
            addMediaItems(currentMediaItemIndex + 1, tracks.map(Track::toMediaItem))
        }
    }

    /** [tracks] at the end of the queue. */
    fun addToQueue(tracks: List<Track>) = command {
        val empty = mediaItemCount == 0
        addMediaItems(tracks.map(Track::toMediaItem))
        if (empty) prepare()
    }

    /** Replaces the queue with [tracks] and plays them shuffled. */
    fun shuffle(tracks: List<Track>) = command {
        setMediaItems(tracks.map(Track::toMediaItem), tracks.indices.random(), 0)
        shuffleModeEnabled = true
        prepare()
        play()
    }

    override fun toggle() = command {
        if (isPlaying) {
            pause()
        } else {
            when (playbackState) {
                Player.STATE_IDLE -> prepare()
                Player.STATE_ENDED -> seekTo(0, 0)
                else -> Unit // ready or buffering: play() is enough
            }
            play()
        }
    }

    override fun next() = command { seekToNext() }

    override fun previous() = command { seekToPrevious() }

    override fun seekTo(positionMs: Long) = command { seekTo(positionMs) }

    override fun jumpTo(index: Int) = command {
        seekTo(index, 0)
        play()
    }

    override fun move(from: Int, to: Int) = command { moveMediaItem(from, to) }

    override fun remove(index: Int) = command { removeMediaItem(index) }

    override fun clearQueue() = command {
        val current = currentMediaItemIndex
        if (current + 1 < mediaItemCount) removeMediaItems(current + 1, mediaItemCount)
        if (current > 0) removeMediaItems(0, current)
    }

    override fun cycleRepeat() = command { repeatMode = nextRepeatMode(repeatMode) }

    override fun toggleShuffle() = command { shuffleModeEnabled = !shuffleModeEnabled }

    private fun command(action: MediaController.() -> Unit) {
        controller?.action() ?: run {
            waiting += action
            connect()
        }
    }

    private val listener = object : Player.Listener {
        override fun onEvents(player: Player, events: Player.Events) {
            refresh(queueChanged = events.contains(Player.EVENT_TIMELINE_CHANGED))
        }
    }

    private fun refresh(queueChanged: Boolean) {
        val c = controller ?: return
        val queue = if (queueChanged) List(c.mediaItemCount) { c.getMediaItemAt(it).toQueueEntry() } else current.value.queue
        current.value = PlaybackState(
            queue = queue,
            index = if (queue.isEmpty()) -1 else c.currentMediaItemIndex,
            isPlaying = c.isPlaying,
            durationMs = c.duration.takeIf { it != C.TIME_UNSET } ?: 0L,
            repeatMode = c.repeatMode,
            shuffle = c.shuffleModeEnabled,
            format = trackFormatOf(c.currentTracks),
        )
    }
}
