package io.github.norbertque.pixeltone.library

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class HomeTest {
    private val tracks = (1L..200L).map { id ->
        Track(id, "T$id", "Artist ${id % 10}", null, "Album ${id % 20}", id % 20, 200_000, null, null, null, "Music/${id % 20}/", "$id.flac", addedAt = id)
    }
    private val index = LibraryIndex(tracks)
    private val now = 100L * 86_400_000

    @Test
    fun `today's mix stays the same all day and changes tomorrow`() {
        val listens = (1..30).map { Listen(ListedTrack.of(tracks[it % 5]), now - it * 3_600_000L, 120_000) }
        val today = dailyMix(index, listens, emptyList(), day = 100, now = now)
        assertEquals(40, today.size)
        assertEquals(today.size, today.distinctBy { it.id }.size)
        assertEquals(today, dailyMix(index, listens, emptyList(), day = 100, now = now))
        assertNotEquals(today, dailyMix(index, listens, emptyList(), day = 101, now = now))
        // Half from the artists played most (Artist 0–4 here).
        assertTrue(today.count { it.artist in (0..4).map { n -> "Artist $n" } } >= 20)
    }

    @Test
    fun `recently added albums come newest first`() {
        // Album 0 holds track 200, the newest file.
        assertEquals("Album 0", recentlyAdded(index).first().title)
    }
}
