package io.github.norbertque.pixeltone.lyrics

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The same rules as Windows' `pp-lyrics` (lrc.rs, lrclib.rs). */
class LyricsTest {
    @Test
    fun `synced lines, several stamps per line, sorted, with the offset applied`() {
        val (lines, synced) = parseLrc(
            """
            [ar:Someone]
            [offset:+500]
            [00:12.30]First
            [00:05.1][01:00.123]Chorus <00:05.50>with <00:06.00>words
            [00:20:50]Colon fraction
            """.trimIndent(),
        )
        assertTrue(synced)
        assertEquals(
            listOf(
                LyricLine(4_600, "Chorus with words"),
                LyricLine(11_800, "First"),
                LyricLine(20_000, "Colon fraction"),
                LyricLine(59_623, "Chorus with words"),
            ),
            lines,
        )
    }

    @Test
    fun `text without stamps is plain, without blank lines at the ends`() {
        val (lines, synced) = parseLrc("\n\nLine one\n\nLine two\n\n")
        assertFalse(synced)
        assertEquals(listOf("Line one", "", "Line two"), lines.map { it.text })
        assertNull(lines.first().timeMs)
        assertEquals(emptyList<LyricLine>(), parseLrc("[ti:Only tags]\n").first)
    }

    @Test
    fun `ID3 USLT frames in each encoding`() {
        fun frame(encoding: Int, description: ByteArray, text: ByteArray) =
            byteArrayOf(encoding.toByte()) + "eng".toByteArray() + description + text
        assertEquals("Hello", uslt(frame(0, byteArrayOf(0), "Hello".toByteArray(Charsets.ISO_8859_1))))
        assertEquals("Příliš", uslt(frame(3, "desc".toByteArray() + 0, "Příliš".toByteArray())))
        assertEquals("Žluť", uslt(frame(1, byteArrayOf(0xFF.toByte(), 0xFE.toByte(), 0, 0), "Žluť".toByteArray(Charsets.UTF_16))))
        assertEquals("Ahoj", uslt(frame(2, byteArrayOf(0, 0), "Ahoj".toByteArray(Charsets.UTF_16BE))))
        assertNull(uslt(byteArrayOf(0, 1)))
    }

    @Test
    fun `LRCLIB's search knows the main artist`() {
        assertEquals("Aurora Lane", mainArtist("Aurora Lane feat. Neon Child"))
        assertEquals("A", mainArtist("A ft. B"))
        assertEquals("Simon & Garfunkel", mainArtist("Simon & Garfunkel"))
    }
}
