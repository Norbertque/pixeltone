package io.github.norbertque.pixeltone.library

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MixesTest {
    private val day = 100L
    private val start = day * DAY_MS

    // 120 tracks: 0–59 Rock (some also Indie), 60–89 Jazz, 90–94 Polka (too few for a mix), the rest untagged.
    private val tracks = (0L until 120L).map { id ->
        val genres = when (id) {
            in 0L until 60L -> if (id % 3 == 0L) listOf("Rock", "Indie") else listOf("rock")
            in 60L until 90L -> listOf("Jazz")
            in 90L until 95L -> listOf("Polka")
            else -> emptyList()
        }
        Track(id, "T$id", "Artist ${id % 7}", null, "Album ${id % 12}", id % 12, 200_000, null, null, null, "Music/${id % 12}/", "$id.flac", genres = genres)
    }
    private val index = LibraryIndex(tracks, Locale.ENGLISH)

    private fun heard(id: Int, at: Long) = Listen(ListedTrack.of(tracks[id]), at, 120_000)

    @Test
    fun `genres are split, told apart without case, and named by the usual spelling`() {
        assertEquals(listOf("Rock", "Pop", "Indie", "Folk"), splitGenres(listOf("Rock; Pop/Indie, Folk", "rock")))
        assertEquals(listOf("Rock"), splitGenres(listOf("(17)Rock", "(17)", "17", " ")))
        assertEquals(listOf("Indie", "Jazz", "Polka", "rock"), index.genres.map { it.name })
        val rock = index.genre("rock")!!
        assertEquals(60, rock.tracks.size)
        assertEquals(12, rock.albums.size)
        assertEquals(listOf("rock"), index.search("ROCK").genres.map { it.key })
    }

    @Test
    fun `a genre's mix only takes its own tracks`() {
        val mixes = todaysMixes(index, emptyList(), emptyList(), day)
        val jazz = mixes.mix("genre:jazz")!!
        assertEquals(30, jazz.tracks.size)
        assertTrue(jazz.tracks.all { "Jazz" in it.genres })
        assertEquals(40, mixes.daily.tracks.size)
        // Polka has too few tracks for a mix.
        assertNull(mixes.mix("genre:polka"))
    }

    @Test
    fun `the genres played most come first, then the biggest`() {
        val listens = (0 until 5).map { heard(60 + it, start - (it + 1) * 3_600_000L) }
        assertEquals(listOf("jazz", "rock", "indie"), mixGenres(index, listens, start).map { it.key })
        assertEquals(listOf("rock", "jazz", "indie"), mixGenres(index, emptyList(), start).map { it.key })
    }

    @Test
    fun `mixes hold all day while today's listening goes on`() {
        val before = (0 until 10).map { heard(it, start - (it + 1) * 3_600_000L) }
        val morning = todaysMixes(index, before, emptyList(), day)
        val evening = todaysMixes(index, before + (20 until 40).map { heard(it, start + it * 60_000L) }, emptyList(), day)
        assertEquals(morning, evening)
        val tomorrow = todaysMixes(index, before, emptyList(), day + 1)
        assertNotEquals(morning.mix("genre:rock"), tomorrow.mix("genre:rock"))
    }
}
