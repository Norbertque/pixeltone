package io.github.norbertque.pixeltone.library

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

/** Windows' cases (`pp-library` artists.rs), and what the library makes of them. */
class ArtistsTest {
    @Test
    fun `splits on the default separators`() {
        assertEquals(listOf("A", "B", "C"), splitArtists("A; B / C"))
        assertEquals(listOf("Daft Punk", "Pharrell Williams"), splitArtists("Daft Punk feat. Pharrell Williams"))
        assertEquals(listOf("Artist A", "Artist B"), splitArtists("Artist A (Feat. Artist B)"))
        assertEquals(listOf("DJ X", "Y"), splitArtists("DJ X Ft Y"))
    }

    @Test
    fun `keeps band names together`() {
        assertEquals(listOf("Simon & Garfunkel"), splitArtists("Simon & Garfunkel"))
        assertEquals(listOf("Crosby, Stills, Nash & Young"), splitArtists("Crosby, Stills, Nash & Young"))
        assertEquals(listOf("AC/DC"), splitArtists("AC/DC"))
        assertEquals(listOf("AC/DC", "Foo"), splitArtists("ac/dc; Foo"))
    }

    @Test
    fun `names once, and the user's separators`() {
        assertEquals(listOf("Björk", "Thom Yorke"), splitArtists("Björk; björk; Thom Yorke"))
        assertEquals(listOf("Simon", "Garfunkel"), splitArtists("Simon & Garfunkel", listOf("&")))
    }

    private val tracks = listOf(
        Track(1, "Get Lucky", "Daft Punk feat. Pharrell Williams", null, "Random Access Memories", 1, 369_000, null, 8, 2013, "Music/RAM/", "8.flac"),
        Track(2, "Giorgio by Moroder", "Daft Punk", null, "Random Access Memories", 1, 544_000, null, 3, 2013, "Music/RAM/", "3.flac"),
        Track(3, "Happy", "Pharrell Williams", null, "G I R L", 2, 233_000, null, 5, 2014, "Music/GIRL/", "5.flac"),
    )
    private val index = LibraryIndex(tracks, Locale.ENGLISH)

    @Test
    fun `a featured artist has the track, and the album stays its artist's`() {
        assertEquals(listOf("Daft Punk", "Pharrell Williams"), index.artists.map { it.name })
        assertEquals(listOf("Get Lucky", "Happy"), index.artistNamed("Pharrell Williams")!!.tracks.map { it.title })
        val ram = index.albums.first { it.title == "Random Access Memories" }
        assertEquals("Daft Punk", ram.artist)
        assertFalse(ram.isCompilation)
        assertEquals("Daft Punk", index.artistNamed("Daft Punk feat. Pharrell Williams")!!.name)
    }

    @Test
    fun `each artist gets the listen`() {
        val listens = tracks.map { Listen(ListedTrack.of(it), 0, it.durationMs) }
        val stats = listeningStats(listens, since = null, artistsOf = index::artistNames)
        assertEquals(listOf("Pharrell Williams" to 2, "Daft Punk" to 2), stats.artists.map { it.item to it.plays }.sortedBy { it.first }.reversed())
        assertEquals(listOf("Random Access Memories", "G I R L"), stats.albums.map { it.item.title })
    }
}
