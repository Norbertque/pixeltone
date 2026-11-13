package io.github.norbertque.pixeltone.playback

import android.os.Bundle
import android.provider.MediaStore
import androidx.media3.common.MediaItem
import androidx.media3.session.MediaConstants
import io.github.norbertque.pixeltone.library.LibraryIndex
import io.github.norbertque.pixeltone.library.ListedTrack
import io.github.norbertque.pixeltone.library.Listen
import io.github.norbertque.pixeltone.library.Lists
import io.github.norbertque.pixeltone.library.Mix
import io.github.norbertque.pixeltone.library.Mixes
import io.github.norbertque.pixeltone.library.Playlist
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.library.albumArtUri
import io.github.norbertque.pixeltone.library.albumIdOf
import java.util.Locale
import kotlin.random.Random
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35])
class CarLibraryTest {
    private val context = RuntimeEnvironment.getApplication()

    private fun track(id: Long, title: String, artist: String, album: String, number: Int, genre: String = "Rock") =
        Track(id, title, artist, artist, album, id / 10, 200_000, 1, number, 2001, "Music/$album/", "$id.flac", addedAt = id, genres = listOf(genre))

    // Two albums of Daft Punk's, one with a slash and a bar in its title, and a jazz album of someone else's.
    private val tracks = listOf(
        track(11, "One More Time", "Daft Punk", "Discovery", 1),
        track(12, "Aerodynamic", "Daft Punk", "Discovery", 2),
        track(13, "Digital Love", "Daft Punk", "Discovery", 3),
        track(21, "Get Lucky", "Daft Punk", "Random Access / Memories | Live", 1),
        track(22, "Lose Yourself to Dance", "Daft Punk", "Random Access / Memories | Live", 2),
        track(31, "So What", "Miles Davis", "Kind of Blue", 1, genre = "Jazz"),
        track(32, "Blue in Green", "Miles Davis", "Kind of Blue", 2, genre = "Jazz"),
    )
    private val index = LibraryIndex(tracks, Locale.ENGLISH)
    private fun listed(id: Long) = ListedTrack.of(tracks.first { it.id == id })
    private val lists = Lists(
        favourites = listOf(listed(31), listed(13)),
        playlists = listOf(Playlist(7, "Road trip", listOf(listed(21), listed(11), listed(21)))),
    )
    private val mixes = Mixes(
        daily = Mix(Mix.DAILY, null, listOf(tracks[5], tracks[0], tracks[3])),
        genres = listOf(Mix("genre:jazz", index.genre("jazz"), listOf(tracks[6], tracks[5]))),
    )
    private val listens = listOf(Listen(listed(31), startedAt = 1_000, heardMs = 120_000))

    private fun car(lists: Lists = this.lists) =
        CarLibrary(context, index, lists, mixes, listens, random = Random(1))

    private val MediaItem.title get() = mediaMetadata.title.toString()
    private val MediaItem.group get() = mediaMetadata.extras?.getString(MediaConstants.EXTRAS_KEY_CONTENT_STYLE_GROUP_TITLE)

    @Test
    fun `the root has the phone's four tabs, all browsable`() {
        val tabs = car().children(CarLibrary.ROOT)!!
        assertEquals(listOf("home", "playlists", "albums", "artists"), tabs.map { it.mediaId })
        assertEquals(listOf("Home", "Playlists", "Albums", "Artists"), tabs.map { it.title })
        assertTrue(tabs.all { it.mediaMetadata.isBrowsable == true && it.mediaMetadata.isPlayable == false })
        val albums = tabs[2].mediaMetadata.extras!!
        assertEquals(MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_GRID_ITEM, albums.getInt(MediaConstants.EXTRAS_KEY_CONTENT_STYLE_BROWSABLE))
        assertEquals("android.resource://${context.packageName}/drawable/ic_car_album", tabs[2].mediaMetadata.artworkUri.toString())
    }

    @Test
    fun `a track picked in an album plays the album from it`() {
        val car = car()
        val album = car.children("albums")!!.first { it.title == "Discovery" }
        val rows = car.children(album.mediaId)!!
        assertEquals(listOf("Shuffle", "One More Time", "Aerodynamic", "Digital Love"), rows.map { it.title })
        assertTrue(rows.all { it.mediaMetadata.isPlayable == true })
        assertEquals(albumArtUri(1), rows[1].mediaMetadata.artworkUri)
        val pick = car.pick(rows[2].mediaId)!!
        assertEquals(listOf(11L, 12L, 13L), pick.tracks.map { it.id })
        assertEquals(1, pick.start)
        assertFalse(pick.shuffle)
        assertEquals("Aerodynamic", car.item(rows[2].mediaId)!!.title)
    }

    @Test
    fun `Shuffle plays the list shuffled from anywhere in it`() {
        val car = car()
        val album = car.children("albums")!!.first { it.title == "Discovery" }
        val pick = car.pick(car.children(album.mediaId)!!.first().mediaId)!!
        assertTrue(pick.shuffle)
        assertEquals(3, pick.tracks.size)
        assertTrue(pick.start in 0..2)
    }

    @Test
    fun `ids hold any album title, slashes and all`() {
        val car = car()
        val album = car.children("albums")!!.first { it.title == "Random Access / Memories | Live" }
        assertTrue(isCarId(album.mediaId))
        assertEquals(1, album.mediaId.count { it == '/' }) // "album/<key>": the key's own slash is escaped
        assertEquals(listOf(21L, 22L), car.pick(album.mediaId)!!.tracks.map { it.id })
        assertEquals("Random Access / Memories | Live", car.item(album.mediaId)!!.title)
        assertFalse(isCarId("21"))
    }

    @Test
    fun `a playlist's rows are told apart even when a track is in it twice`() {
        val car = car()
        val rows = car.children("playlist/7")!!.drop(1)
        assertEquals(listOf("Get Lucky", "One More Time", "Get Lucky"), rows.map { it.title })
        assertEquals(3, rows.map { it.mediaId }.distinct().size)
        assertEquals(2, car.pick(rows[2].mediaId)!!.start)
        assertEquals(listOf(21L), car.tracks(rows[2].mediaId).map { it.id })
        assertEquals(3, car.tracks("playlist/7").size)
    }

    @Test
    fun `a track gone from its list since still plays, alone`() {
        val shown = car().children("favourites")!!.first { it.title == "Digital Love" }
        val pick = car(lists = lists.copy(favourites = listOf(listed(31)))).pick(shown.mediaId)!!
        assertEquals(listOf(13L), pick.tracks.map { it.id })
        assertEquals(listOf(11L), car().pick("playlist/99/0-11")!!.tracks.map { it.id })
        assertNull(car().pick("nothing/at/all"))
    }

    @Test
    fun `Playlists are favourites first, then the playlists`() {
        val rows = car().children("playlists")!!
        assertEquals(listOf("Favourites", "Road trip"), rows.map { it.title })
        assertEquals(listOf("2 tracks", "3 tracks"), rows.map { it.mediaMetadata.subtitle.toString() })
    }

    @Test
    fun `Home is Shuffle all, today's mixes, and recently played and added albums, each album once`() {
        val rows = car().children("home")!!
        assertEquals("Shuffle all", rows[0].title)
        assertEquals("all/shuffle", rows[0].mediaId)
        assertEquals(listOf("Today’s mix", "Jazz mix"), rows.filter { it.group == "Made for you" }.map { it.title })
        assertEquals(listOf("Kind of Blue"), rows.filter { it.group == "Recently played" }.map { it.title })
        val added = rows.filter { it.group == "Recently added" }.map { it.title }
        assertEquals(listOf("Random Access / Memories | Live", "Discovery"), added)
        assertEquals(rows.size, rows.map { it.mediaId }.distinct().size)
        assertEquals(7, car().pick("all/shuffle")!!.tracks.size)
    }

    @Test
    fun `an artist shows their albums, then their tracks`() {
        val car = car()
        val artist = car.children("artists")!!.first { it.title == "Daft Punk" }
        assertEquals("2 albums · 5 tracks", artist.mediaMetadata.subtitle.toString())
        val rows = car.children(artist.mediaId)!!
        assertEquals(listOf("Discovery", "Random Access / Memories | Live"), rows.filter { it.group == "Albums" }.map { it.title })
        val tracks = rows.filter { it.group == "Tracks" }
        assertEquals("Shuffle", tracks.first().title)
        assertEquals(6, tracks.size)
    }

    @Test
    fun `search finds tracks, albums, artists and playlists, each under its heading`() {
        val car = car()
        val found = car.search("lucky")
        assertEquals(listOf("Get Lucky"), found.map { it.title })
        assertEquals("Tracks", found.single().group)
        assertEquals(listOf(21L), car.pick(found.single().mediaId)!!.tracks.map { it.id })
        assertEquals(listOf("Albums"), car.search("kind blue").map { it.group }.distinct().filter { it != "Tracks" })
        assertEquals(listOf("Road trip"), car.search("road").map { it.title })
        assertTrue(car.search("  ").isEmpty())
    }

    @Test
    fun `spoken requests play what was named`() {
        val car = car()
        assertEquals(listOf(31L, 11L, 21L), car.spoken(SpokenRequest(""))!!.tracks.map { it.id })
        assertEquals(listOf(31L, 13L), car.spoken(SpokenRequest("favorites"))!!.tracks.map { it.id })
        assertEquals(listOf(32L, 31L), car.spoken(SpokenRequest("jazz mix"))!!.tracks.map { it.id })
        assertEquals(listOf(21L, 11L, 21L), car.spoken(SpokenRequest("road trip"))!!.tracks.map { it.id })
        val album = car.spoken(SpokenRequest("discovery"))!!
        assertEquals(CarPick(tracks.take(3), 0, shuffle = false), album)
        val artist = car.spoken(SpokenRequest("Miles Davis"))!!
        assertTrue(artist.shuffle)
        assertEquals(setOf(31L, 32L), artist.tracks.map { it.id }.toSet())
        // A track goes on with the rest of its album.
        assertEquals(CarPick(tracks.take(3), 2, shuffle = false), car.spoken(SpokenRequest("digital love")))
        assertEquals(CarPick(tracks.subList(3, 5), 0, shuffle = false), car.spoken(SpokenRequest("get lucky by daft punk")))
        assertEquals(listOf(11L), car.spoken(SpokenRequest("one more"))!!.tracks.map { it.id })
        assertNull(car.spoken(SpokenRequest("polka")))
    }

    @Test
    fun `what the assistant made of a request comes first`() {
        val car = car()
        val focused = SpokenRequest("blue", focus = MediaStore.Audio.Albums.ENTRY_CONTENT_TYPE, album = "Kind of Blue")
        assertEquals(listOf(31L, 32L), car.spoken(focused)!!.tracks.map { it.id })
        val track = SpokenRequest("x", focus = MediaStore.Audio.Media.ENTRY_CONTENT_TYPE, title = "Get Lucky", artist = "Daft Punk")
        assertEquals(21L, car.spoken(track)!!.let { it.tracks[it.start].id })
        val extras = Bundle().apply {
            putString(MediaStore.EXTRA_MEDIA_FOCUS, MediaStore.Audio.Artists.ENTRY_CONTENT_TYPE)
            putString(MediaStore.EXTRA_MEDIA_ARTIST, "Daft Punk")
        }
        val item = MediaItem.Builder()
            .setRequestMetadata(MediaItem.RequestMetadata.Builder().setSearchQuery("daft punk").setExtras(extras).build())
            .build()
        val request = spokenRequestOf(item)!!
        assertEquals(MediaStore.Audio.Artists.ENTRY_CONTENT_TYPE, request.focus)
        assertEquals(5, car.spoken(request)!!.tracks.size)
        assertNull(spokenRequestOf(MediaItem.Builder().setMediaId("11").build()))
    }

    @Test
    fun `covers are the app's own addresses, and MediaStore's still read`() {
        val uri = albumArtUri(42)
        assertEquals("content://${context.packageName}.covers/album/42", uri.toString())
        assertEquals(42L, albumIdOf(uri))
        assertEquals(7L, albumIdOf(android.net.Uri.parse("content://media/external/audio/albumart/7")))
        assertNull(albumIdOf(android.net.Uri.parse("content://elsewhere/album/7")))
        assertNotNull(car().item("favourites"))
    }
}
