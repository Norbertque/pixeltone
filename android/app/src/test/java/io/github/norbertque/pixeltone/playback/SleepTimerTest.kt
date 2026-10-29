package io.github.norbertque.pixeltone.playback

import org.junit.Assert.assertEquals
import org.junit.Test

class SleepTimerTest {
    @Test
    fun `minutes from now, end of track, and off`() {
        var now = 1_000L
        val timer = SleepTimer { now }
        assertEquals(Sleep.Off, timer.state.value)
        timer.inMinutes(15)
        assertEquals(Sleep.At(1_000 + 15 * 60_000L), timer.state.value)
        now += 5_000
        timer.inMinutes(30)
        assertEquals(Sleep.At(6_000 + 30 * 60_000L), timer.state.value)
        timer.atEndOfTrack()
        assertEquals(Sleep.EndOfTrack, timer.state.value)
        timer.cancel()
        assertEquals(Sleep.Off, timer.state.value)
    }
}
