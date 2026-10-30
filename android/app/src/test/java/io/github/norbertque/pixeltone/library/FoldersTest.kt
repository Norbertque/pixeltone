package io.github.norbertque.pixeltone.library

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Test

class FoldersTest {
    private var id = 0L

    private fun at(folder: String, file: String) = Track(++id, file, "A", null, "X", 1, 200_000, null, null, null, folder, file)

    @Test
    fun `folders list their folders with every track under them, then their own tracks`() {
        val index = LibraryIndex(
            listOf(
                at("Music/Kryštof/Rubikon/", "02 Ty a já.flac"),
                at("Music/Kryštof/Rubikon/", "01 Inzerát.flac"),
                at("Music/Kryštof/Inzerát/", "01 Cesta.flac"),
                at("Music/", "loose.mp3"),
                at("Download/", "song.mp3"),
            ),
            Locale.ENGLISH,
        )
        assertEquals("", index.rootFolder)
        assertEquals(listOf("Download" to 1, "Music" to 4), index.folder("").folders.map { it.name to it.tracks })
        val music = index.folder("Music")
        assertEquals(listOf(FolderEntry("Music/Kryštof", "Kryštof", 3)), music.folders)
        assertEquals(listOf("loose.mp3"), music.tracks.map { it.fileName })
        assertEquals(listOf("01 Inzerát.flac", "02 Ty a já.flac"), index.folder("Music/Kryštof/Rubikon").tracks.map { it.fileName })
        assertEquals(listOf("Download", "Music", "Music/Kryštof", "Music/Kryštof/Inzerát", "Music/Kryštof/Rubikon"), index.everyFolder.map { it.path })
    }

    @Test
    fun `a path every file shares isn't clicked through`() {
        val index = LibraryIndex(
            listOf(at("/storage/emulated/0/Music/A", "1.flac"), at("/storage/emulated/0/Download", "2.flac")),
            Locale.ENGLISH,
        )
        assertEquals("storage/emulated/0", index.rootFolder)
        assertEquals(listOf("Download", "Music"), index.folder(index.rootFolder).folders.map { it.name })
    }
}
