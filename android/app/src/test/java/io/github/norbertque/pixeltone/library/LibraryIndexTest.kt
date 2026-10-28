package io.github.norbertque.pixeltone.library

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LibraryIndexTest {
    private var nextId = 1L

    private fun track(
        title: String,
        artist: String? = "Artist",
        album: String? = "Album",
        albumArtist: String? = null,
        folder: String = "Music/",
        disc: Int? = null,
        number: Int? = null,
        year: Int? = null,
    ) = Track(nextId++, title, artist, albumArtist, album, 7, 1_000, disc, number, year, folder)

    @Test
    fun `albums are told apart by album artist, else by folder`() {
        val index = LibraryIndex(
            listOf(
                track("A", album = "Greatest Hits", albumArtist = "Queen", folder = "x/"),
                track("B", album = "Greatest Hits", albumArtist = "QUEEN", folder = "y/"),
                track("C", album = "Greatest Hits", albumArtist = "ABBA"),
                track("D", album = "Untitled", folder = "one/"),
                track("E", album = "Untitled", folder = "two/"),
            ),
            Locale.ENGLISH,
        )
        assertEquals(4, index.albums.size)
        assertEquals(listOf("A", "B"), index.albums.first { it.artist == "Queen" }.tracks.map { it.title })
    }

    @Test
    fun `album tracks follow disc and track numbers`() {
        val index = LibraryIndex(
            listOf(
                track("Last", disc = 2, number = 1),
                track("Second", disc = 1, number = 2),
                track("First", number = 1),
                track("Loose"),
            ),
            Locale.ENGLISH,
        )
        assertEquals(listOf("First", "Second", "Loose", "Last"), index.albums.single().tracks.map { it.title })
    }

    @Test
    fun `an album without album artist takes the artist its tracks share`() {
        val index = LibraryIndex(
            listOf(
                track("A", artist = "Björk", album = "Debut"),
                track("B", artist = "BJORK", album = "Debut"),
                track("C", artist = "X", album = "Mix"),
                track("D", artist = "Y", album = "Mix"),
            ),
            Locale.ENGLISH,
        )
        val debut = index.albums.first { it.title == "Debut" }
        assertEquals("Björk", debut.artist)
        assertFalse(debut.isCompilation)
        val mix = index.albums.first { it.title == "Mix" }
        assertNull(mix.artist)
        assertTrue(mix.isCompilation)
    }

    @Test
    fun `artists come from track and album artists, in the language's order`() {
        val index = LibraryIndex(
            listOf(
                track("A", artist = "Zorro", album = "Z"),
                track("B", artist = "Čechomor", album = "C"),
                track("C", artist = "Dan", album = "Hits", albumArtist = "Various Names"),
            ),
            Locale.forLanguageTag("cs"),
        )
        assertEquals(listOf("Čechomor", "Dan", "Various Names", "Zorro"), index.artists.map { it.name })
        assertEquals(listOf("Hits"), index.artistNamed("various names")!!.albums.map { it.title })
        assertEquals(listOf("Hits"), index.artistNamed("Dan")!!.albums.map { it.title })
    }

    @Test
    fun `search ignores case and accents, and every word must match`() {
        val index = LibraryIndex(
            listOf(
                track("Příběh", artist = "Kryštof", album = "Rubikon"),
                track("Story", artist = "Someone", album = "Rubikon 2"),
            ),
            Locale.ENGLISH,
        )
        assertEquals(listOf("Příběh"), index.search("PRIBEH").tracks.map { it.title })
        assertEquals(listOf("Příběh"), index.search(" krystof  rubikon ").tracks.map { it.title })
        assertTrue(index.search("krystof story").tracks.isEmpty())
        assertEquals(listOf("Kryštof"), index.search("kryst").artists.map { it.name })
        assertEquals(2, index.search("  ").tracks.size)
    }

    @Test
    fun `tracks without an album are listed but make no album`() {
        val index = LibraryIndex(listOf(track("Single", album = null)), Locale.ENGLISH)
        assertEquals(1, index.tracks.size)
        assertTrue(index.albums.isEmpty())
        assertEquals(1, index.artists.single().tracks.size)
    }
}
