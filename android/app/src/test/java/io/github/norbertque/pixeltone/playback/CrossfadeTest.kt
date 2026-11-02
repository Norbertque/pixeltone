package io.github.norbertque.pixeltone.playback

import org.junit.Assert.assertEquals
import org.junit.Test

class CrossfadeTest {
    private val sound = Sound(crossfadeMs = 6_000)

    private fun step(
        sound: Sound = this.sound,
        positionMs: Long,
        durationMs: Long = 200_000,
        playing: Boolean = true,
        hasNext: Boolean = true,
        repeatsOne: Boolean = false,
        stopsAtEnd: Boolean = false,
        albums: Pair<String?, String?> = "A" to "B",
    ) = crossfadeStep(sound, playing, positionMs, durationMs, hasNext, repeatsOne, stopsAtEnd, albums.first, albums.second)

    @Test
    fun `the next track gets ready, then starts the crossfade's length before the end`() {
        assertEquals(CrossfadeStep.None, step(positionMs = 100_000))
        assertEquals(CrossfadeStep.Prepare, step(positionMs = 191_000))
        assertEquals(CrossfadeStep.Start(6_000), step(positionMs = 194_000))
        assertEquals(CrossfadeStep.Start(2_500), step(positionMs = 197_500))
    }

    @Test
    fun `no crossfade when off, paused, at the end, repeating one or stopping after the track`() {
        assertEquals(CrossfadeStep.None, step(Sound(), positionMs = 197_000))
        assertEquals(CrossfadeStep.None, step(positionMs = 197_000, playing = false))
        assertEquals(CrossfadeStep.None, step(positionMs = 197_000, hasNext = false))
        assertEquals(CrossfadeStep.None, step(positionMs = 197_000, repeatsOne = true))
        assertEquals(CrossfadeStep.None, step(positionMs = 197_000, stopsAtEnd = true))
    }

    @Test
    fun `albums stay gapless unless they may be crossfaded too`() {
        assertEquals(CrossfadeStep.None, step(positionMs = 197_000, albums = "Rubikon" to "Rubikon"))
        assertEquals(CrossfadeStep.Start(3_000), step(sound.copy(crossfadeAlbums = true), positionMs = 197_000, albums = "Rubikon" to "Rubikon"))
        // Tracks without an album aren't one album.
        assertEquals(CrossfadeStep.Start(3_000), step(positionMs = 197_000, albums = null to null))
    }

    @Test
    fun `a short track fades over half its length at most`() {
        assertEquals(CrossfadeStep.Start(4_000), step(positionMs = 4_000, durationMs = 8_000))
    }

    @Test
    fun `the fade keeps the power even`() {
        val (out0, in0) = crossfadeVolumes(0f)
        assertEquals(1f, out0, 1e-6f)
        assertEquals(0f, in0, 1e-6f)
        val (outHalf, inHalf) = crossfadeVolumes(0.5f)
        assertEquals(1f, outHalf * outHalf + inHalf * inHalf, 1e-5f)
        assertEquals(outHalf, inHalf, 1e-6f)
        assertEquals(0f to 1f, crossfadeVolumes(1f).let { (a, b) -> (if (a < 1e-6f) 0f else a) to b })
    }
}
