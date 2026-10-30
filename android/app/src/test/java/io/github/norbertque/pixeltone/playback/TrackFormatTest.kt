package io.github.norbertque.pixeltone.playback

import androidx.media3.common.C
import androidx.media3.common.Format
import androidx.media3.common.MimeTypes
import androidx.media3.common.TrackGroup
import androidx.media3.common.Tracks
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class TrackFormatTest {
    private fun playing(format: Format) =
        Tracks(listOf(Tracks.Group(TrackGroup(format), false, intArrayOf(C.FORMAT_HANDLED), booleanArrayOf(true))))

    @Test
    fun `lossless shows its depth and rate`() {
        val flac = Format.Builder().setSampleMimeType(MimeTypes.AUDIO_FLAC).setPcmEncoding(C.ENCODING_PCM_24BIT).setSampleRate(96_000).build()
        assertEquals("FLAC • 24-bit/96 kHz", trackFormatOf(playing(flac))?.label)
    }

    @Test
    fun `lossy shows its bitrate and rate`() {
        val mp3 = Format.Builder().setSampleMimeType(MimeTypes.AUDIO_MPEG).setAverageBitrate(320_000).setSampleRate(44_100).build()
        assertEquals("MP3 • 320 kbps • 44.1 kHz", trackFormatOf(playing(mp3))?.label)
        val aac = Format.Builder().setSampleMimeType(MimeTypes.AUDIO_AAC).setSampleRate(48_000).build()
        assertEquals("AAC • 48 kHz", trackFormatOf(playing(aac))?.label)
    }

    @Test
    fun `nothing playing, nothing shown`() {
        assertNull(trackFormatOf(Tracks.EMPTY))
    }
}
