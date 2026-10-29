package io.github.norbertque.pixeltone.library

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class UserListsTest {
    private var nextId = 1L

    private fun track(title: String, file: String = "$title.flac", folder: String = "Music/Album/") =
        Track(nextId++, title, "Artist", null, "Album", 7, 1_000, null, null, null, folder, file)

    private fun file() = File.createTempFile("lists", ".json").apply { delete() }

    @Test
    fun `favourites and playlists survive a restart, newest favourite first`() {
        val where = file()
        val a = track("A")
        val b = track("B")
        val lists = UserLists(where)
        lists.setFavourite(a, true)
        lists.setFavourite(b, true)
        val id = lists.create("  Road trip ", listOf(a, b, a))
        lists.move(id, 0, 2)
        lists.removeAt(id, 0)
        lists.setFavourite(a, false)

        // The writes go one after another in the background.
        val deadline = System.currentTimeMillis() + 5_000
        var reread = UserLists(where).lists.value
        while (reread != lists.lists.value && System.currentTimeMillis() < deadline) {
            Thread.sleep(20)
            reread = UserLists(where).lists.value
        }
        assertEquals(lists.lists.value, reread)
        assertEquals(listOf("B"), reread.favourites.map { it.title })
        assertTrue(reread.isFavourite(b))
        assertFalse(reread.isFavourite(a))
        val playlist = reread.playlist(id)!!
        assertEquals("Road trip", playlist.name)
        assertEquals(listOf("A", "A"), playlist.tracks.map { it.title })
    }

    @Test
    fun `lists remember files, which outlive MediaStore's ids`() {
        val lists = UserLists(file())
        val before = track("A", folder = "Music/Album/")
        lists.setFavourite(before, true)
        // After a rescan the same file comes back under another id.
        val after = before.copy(id = 999)
        assertTrue(lists.lists.value.isFavourite(after))
        val index = LibraryIndex(listOf(after))
        assertEquals(999L, index.trackByFile(lists.lists.value.favourites.single().file)?.id)
        assertNull(index.trackByFile("Music/Other/A.flac"))
    }

    @Test
    fun `renamed and deleted playlists`() {
        val lists = UserLists(file())
        val first = lists.create("One")
        val second = lists.create("Two", listOf(track("X")))
        lists.rename(first, "Uno")
        lists.delete(second)
        assertEquals(listOf("Uno"), lists.lists.value.playlists.map { it.name })
        assertTrue(first != second)
        // Out-of-range edits do nothing.
        lists.move(first, 0, 3)
        lists.removeAt(first, 5)
        assertEquals(emptyList<ListedTrack>(), lists.lists.value.playlist(first)!!.tracks)
    }
}
