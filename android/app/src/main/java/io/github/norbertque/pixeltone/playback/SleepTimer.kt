package io.github.norbertque.pixeltone.playback

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** When playback should stop by itself. */
sealed interface Sleep {
    data object Off : Sleep

    /** At [endsAt] (epoch ms). */
    data class At(val endsAt: Long) : Sleep

    /** When the track that plays now ends. */
    data object EndOfTrack : Sleep
}

/** The sleep timer's setting; the playback service carries it out (also with the app closed). */
class SleepTimer(private val clock: () -> Long = System::currentTimeMillis) {
    private val current = MutableStateFlow<Sleep>(Sleep.Off)
    val state: StateFlow<Sleep> = current.asStateFlow()

    fun inMinutes(minutes: Int) {
        current.value = Sleep.At(clock() + minutes * 60_000L)
    }

    fun atEndOfTrack() {
        current.value = Sleep.EndOfTrack
    }

    fun cancel() {
        current.value = Sleep.Off
    }
}
