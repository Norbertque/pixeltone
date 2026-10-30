package io.github.norbertque.pixeltone.library

import io.github.norbertque.pixeltone.ui.settings.LibraryFolder
import io.github.norbertque.pixeltone.ui.settings.libraryFolders
import org.junit.Assert.assertEquals
import org.junit.Test

class LibrarySettingsTest {
    private val tracks = listOf(
        Track(1, "Song", "A", null, "X", 1, 200_000, null, null, null, "Music/X/", "1.flac"),
        Track(2, "Interlude", "A", null, "X", 1, 20_000, null, null, null, "Music/X/", "2.flac"),
        Track(3, "Voice note", null, null, null, 2, 8_000, null, null, null, "WhatsApp Audio/", "3.opus"),
        Track(4, "Demo", "B", null, null, 3, 95_000, null, null, null, "Recordings/", "4.m4a"),
    )

    @Test
    fun `short files and hidden folders stay out`() {
        assertEquals(listOf(1L, 2L, 3L, 4L), shownTracks(tracks, 0, emptySet()).map { it.id })
        assertEquals(listOf(1L, 4L), shownTracks(tracks, 30_000, emptySet()).map { it.id })
        assertEquals(listOf(1L), shownTracks(tracks, 30_000, setOf("Recordings/")).map { it.id })
    }

    @Test
    fun `folders come by path with their counts`() {
        assertEquals(
            listOf(LibraryFolder("Music/X/", 2), LibraryFolder("Recordings/", 1), LibraryFolder("WhatsApp Audio/", 1)),
            libraryFolders(tracks),
        )
    }
}
