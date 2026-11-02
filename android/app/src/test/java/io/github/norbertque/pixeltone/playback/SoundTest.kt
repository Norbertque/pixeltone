package io.github.norbertque.pixeltone.playback

import androidx.media3.common.C
import androidx.media3.common.Metadata
import androidx.media3.common.audio.AudioProcessor.AudioFormat
import androidx.media3.extractor.metadata.id3.InternalFrame
import androidx.media3.extractor.metadata.id3.TextInformationFrame
import androidx.media3.extractor.metadata.vorbis.VorbisComment
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.sin
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SoundTest {
    private val tags = ReplayGainTags(trackGainDb = -6f, albumGainDb = -8f)

    @Test
    fun `replay gain picks the mode, falls back and respects the peak, as on Windows`() {
        assertNull(replayGainDb(ReplayGainMode.Off, 0f, tags))
        assertEquals(-6f, replayGainDb(ReplayGainMode.Track, 0f, tags))
        assertEquals(-8f, replayGainDb(ReplayGainMode.Album, 0f, tags))
        assertEquals(-6f, replayGainDb(ReplayGainMode.Album, 0f, ReplayGainTags(trackGainDb = -6f)))
        assertEquals(-4f, replayGainDb(ReplayGainMode.Track, 2f, tags))
        assertNull(replayGainDb(ReplayGainMode.Track, 0f, ReplayGainTags()))
        // +3 dB asked, but the peak (0.9 ≈ −0.92 dBFS) only allows +0.92 dB.
        val boost = replayGainDb(ReplayGainMode.Track, 0f, ReplayGainTags(trackGainDb = 3f, trackPeak = 0.9f))!!
        assertEquals(0.915f, boost, 0.01f)
    }

    @Test
    fun `tags come from Vorbis comments, ID3 and iTunes entries, and Opus gains`() {
        val flac = Metadata(
            VorbisComment("REPLAYGAIN_TRACK_GAIN", "-6.50 dB"),
            VorbisComment("replaygain_track_peak", "0.950000"),
            VorbisComment("REPLAYGAIN_ALBUM_GAIN", "-7.25 dB"),
        )
        assertEquals(ReplayGainTags(-6.5f, 0.95f, -7.25f, null), replayGainTagsOf(flac))
        val mp3 = Metadata(TextInformationFrame("TXXX", "replaygain_track_gain", listOf("+2.10 dB")))
        assertEquals(2.1f, replayGainTagsOf(mp3).trackGainDb!!, 0.001f)
        val m4a = Metadata(InternalFrame("com.apple.iTunes", "replaygain_album_gain", "-3.10 dB"))
        assertEquals(-3.1f, replayGainTagsOf(m4a).albumGainDb!!, 0.001f)
        // −10 dB towards −23 LUFS is −5 dB towards ReplayGain's −18.
        val opus = Metadata(VorbisComment("R128_TRACK_GAIN", "-2560"))
        assertEquals(-5f, replayGainTagsOf(opus).trackGainDb!!, 0.001f)
        assertEquals(ReplayGainTags(), replayGainTagsOf(null))
    }

    @Test
    fun `a band moved makes the curve custom, unless it matches a preset again`() {
        val rock = EqSettings(enabled = true).withPreset("Rock")
        assertEquals("Rock", rock.preset)
        assertEquals(-4f, rock.preampDb)
        val moved = rock.withBand(5, 2.3f)
        assertEquals(EQ_CUSTOM, moved.preset)
        assertEquals(2.5f, moved.gainsDb[5])
        assertEquals("Rock", moved.withBand(5, 0f).preset)
        assertEquals(EQ_MAX_DB, rock.withBand(0, 40f).gainsDb[0])
        assertTrue(rock.active)
        assertTrue(!EqSettings(enabled = true).active)
    }

    private fun sine(hz: Double, frames: Int, rate: Int = 48_000) = DoubleArray(frames) { 0.5 * sin(2 * PI * hz * it / rate) }

    private fun peak(samples: DoubleArray, from: Int) = samples.drop(from).maxOf { abs(it) }

    @Test
    fun `a boosted band stays level at its centre while the rest makes room`() {
        val eq = Equalizer(48_000, 1).apply { set(EqSettings(enabled = true).withBand(5, 6f)) }
        val atBand = sine(1_000.0, 48_000).also { eq.process(it, it.size) }
        assertEquals(0.5, peak(atBand, 24_000), 0.02)
        val below = Equalizer(48_000, 1).apply { set(EqSettings(enabled = true).withBand(5, 6f)) }
        val low = sine(100.0, 48_000).also { below.process(it, it.size) }
        assertEquals(0.25, peak(low, 24_000), 0.02)
    }

    private fun shorts(vararg values: Short): ByteBuffer =
        ByteBuffer.allocateDirect(values.size * 2).order(ByteOrder.nativeOrder()).apply {
            values.forEach { putShort(it) }
            flip()
        }

    private fun SoundProcessor.run(input: ByteBuffer): List<Short> {
        queueInput(input)
        val out = output
        return List(out.remaining() / 2) { out.getShort() }
    }

    @Test
    fun `the processor leaves samples alone when off and takes a track's gain where it starts`() {
        var sound = Sound()
        val processor = SoundProcessor({ sound })
        processor.configure(AudioFormat(44_100, 2, C.ENCODING_PCM_16BIT))
        processor.flush()
        assertEquals(listOf<Short>(1000, -1000, 32_767, -32_768), processor.run(shorts(1000, -1000, 32_767, -32_768)))

        sound = Sound(replayGain = ReplayGainMode.Track)
        processor.nextTags = ReplayGainTags(trackGainDb = -6.0206f)
        // Still the last track's samples: no tags, no gain.
        assertEquals(listOf<Short>(1000, -1000), processor.run(shorts(1000, -1000)))
        processor.flush()
        assertEquals(listOf<Short>(500, -500), processor.run(shorts(1000, -1000)))
    }
}
