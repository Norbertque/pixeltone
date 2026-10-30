package io.github.norbertque.pixeltone.playback

import android.content.Context
import androidx.core.content.edit
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** How playback behaves, as chosen in Settings; kept in the app's preferences. */
class PlaybackSettings(context: Context) {
    private val prefs = context.getSharedPreferences("playback", Context.MODE_PRIVATE)
    private val unplug = MutableStateFlow(prefs.getBoolean(PAUSE_ON_UNPLUG, true))
    private val swiped = MutableStateFlow(prefs.getBoolean(STOP_WHEN_SWIPED, false))
    private val queueUp = MutableStateFlow(prefs.getBoolean(QUEUE_FIRST, false))

    /** Headphones coming out pause the music (Android's "becoming noisy"). */
    val pauseOnUnplug: StateFlow<Boolean> = unplug.asStateFlow()

    /** Swiping Pixeltone away from the recent apps stops the music. */
    val stopWhenSwipedAway: StateFlow<Boolean> = swiped.asStateFlow()

    /** The panel under Now Playing comes up with the queue rather than the lyrics. */
    val queueFirst: StateFlow<Boolean> = queueUp.asStateFlow()

    fun setPauseOnUnplug(on: Boolean) = set(PAUSE_ON_UNPLUG, unplug, on)

    fun setStopWhenSwipedAway(on: Boolean) = set(STOP_WHEN_SWIPED, swiped, on)

    fun setQueueFirst(on: Boolean) = set(QUEUE_FIRST, queueUp, on)

    private fun set(key: String, flow: MutableStateFlow<Boolean>, on: Boolean) {
        prefs.edit { putBoolean(key, on) }
        flow.value = on
    }

    private companion object {
        const val PAUSE_ON_UNPLUG = "pause_on_unplug"
        const val STOP_WHEN_SWIPED = "stop_when_swiped_away"
        const val QUEUE_FIRST = "queue_first"
    }
}
