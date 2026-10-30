package io.github.norbertque.pixeltone.library

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Test

class SortingTest {
    private val tracks = listOf(
        Track(1, "Cesta", "Kryštof", null, "Inzerát", 1, 236_000, null, 1, 2003, "Music/I/", "1.flac", addedAt = 30),
        Track(2, "Hey Jude", "The Beatles", null, "1", 2, 431_000, null, 7, 2000, "Music/1/", "7.flac", addedAt = 10),
        Track(3, "Inzerát", "Kryštof", null, "Rubikon", 3, 214_000, null, 1, 2004, "Music/R/", "1.flac", addedAt = 20),
        Track(4, "Untagged", null, null, null, 4, 61_000, null, null, null, "Music/", "x.mp3", addedAt = 40),
    )
    private val index = LibraryIndex(tracks, Locale.ENGLISH)

    private fun order(sort: TrackSort) = sortTracks(index.tracks, sort, Locale.ENGLISH).map { it.title }

    @Test
    fun `tracks come in Windows' orders`() {
        assertEquals(listOf("Cesta", "Hey Jude", "Inzerát", "Untagged"), order(TrackSort.Title))
        assertEquals(listOf("Cesta", "Inzerát", "Hey Jude", "Untagged"), order(TrackSort.Artist))
        assertEquals(listOf("Hey Jude", "Cesta", "Inzerát", "Untagged"), order(TrackSort.Album))
        assertEquals(listOf("Inzerát", "Cesta", "Hey Jude", "Untagged"), order(TrackSort.Year))
        assertEquals(listOf("Untagged", "Cesta", "Inzerát", "Hey Jude"), order(TrackSort.Added))
        assertEquals(listOf("Hey Jude", "Cesta", "Inzerát", "Untagged"), order(TrackSort.Duration))
    }

    @Test
    fun `albums come in Windows' orders`() {
        fun order(sort: AlbumSort) = sortAlbums(index.albums, sort, Locale.ENGLISH).map { it.title }
        assertEquals(listOf("1", "Inzerát", "Rubikon"), order(AlbumSort.Title))
        assertEquals(listOf("Inzerát", "Rubikon", "1"), order(AlbumSort.Artist))
        assertEquals(listOf("Rubikon", "Inzerát", "1"), order(AlbumSort.Year))
        assertEquals(listOf("Inzerát", "Rubikon", "1"), order(AlbumSort.Added))
    }
}
