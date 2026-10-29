package io.github.norbertque.pixeltone.library

import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

/** One listen that counted: what played, when it started, and how long it was heard. */
@Serializable
data class Listen(
    val track: ListedTrack,
    val startedAt: Long,
    val heardMs: Long,
)

/**
 * Turns playback into listens. A listen counts as on Windows (`src-tauri/src/history.rs`):
 * after half the track or 4 minutes, whichever is first, and never under 30 seconds.
 */
class PlayCounter(private val clock: () -> Long, private val record: (Listen) -> Unit) {
    private class Current(val track: Track, val startedAt: Long) {
        var heardMs = 0L
        var playingSince: Long? = null
    }

    private var current: Current? = null

    /**
     * [track] is the one playing now (null: none). [again]: the same track starting over (repeat
     * one, previous), a new listen; otherwise the same track carries on (a rebuilt player).
     */
    @Synchronized
    fun onTrack(track: Track?, playing: Boolean, again: Boolean = false) {
        val now = clock()
        val listening = current
        if (listening != null && track != null && !again && listening.track.fileKey == track.fileKey) return
        finish(now)
        current = track?.let { Current(it, now).apply { if (playing) playingSince = now } }
    }

    @Synchronized
    fun onPlaying(playing: Boolean) {
        val listening = current ?: return
        val now = clock()
        val since = listening.playingSince
        when {
            playing && since == null -> listening.playingSince = now
            !playing && since != null -> {
                listening.heardMs += now - since
                listening.playingSince = null
            }
        }
    }

    /** The app stops: a listen that already counts is kept. */
    @Synchronized
    fun flush() {
        finish(clock())
    }

    private fun finish(now: Long) {
        val listening = current ?: return
        current = null
        val heard = listening.heardMs + (listening.playingSince?.let { now - it } ?: 0L)
        val needed = if (listening.track.durationMs > 0) minOf(listening.track.durationMs / 2, ENOUGH_MS) else ENOUGH_MS
        if (heard >= MIN_LISTEN_MS && heard >= needed) {
            record(Listen(ListedTrack.of(listening.track), listening.startedAt, heard))
        }
    }

    companion object {
        const val MIN_LISTEN_MS = 30_000L
        const val ENOUGH_MS = 240_000L
    }
}

/** The play history, one JSON line per listen in the app's files, appended in the background. */
class History(private val file: File) {
    private val writes = CoroutineScope(SupervisorJob() + Dispatchers.IO.limitedParallelism(1))
    private val json = Json { ignoreUnknownKeys = true }
    private val current = MutableStateFlow(load())

    /** Oldest first. */
    val listens: StateFlow<List<Listen>> = current.asStateFlow()

    fun record(listen: Listen) {
        current.update { it + listen }
        writes.launch {
            file.parentFile?.mkdirs()
            file.appendText(json.encodeToString(Listen.serializer(), listen) + "\n")
        }
    }

    private fun load(): List<Listen> =
        runCatching {
            file.readLines().mapNotNull { line ->
                line.takeIf { it.isNotBlank() }?.let { runCatching { json.decodeFromString(Listen.serializer(), it) }.getOrNull() }
            }
        }.getOrDefault(emptyList())
}
