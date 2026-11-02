package io.github.norbertque.pixeltone.library

import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress
import java.net.URLDecoder
import java.util.concurrent.CopyOnWriteArrayList
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Windows' cases (`pp-deezer`), and a search against a stand-in for Deezer. */
class ArtistPicturesTest {
    private fun hit(title: String, id: Long, artist: String, xl: String) = DeezerHit(title, DeezerArtist(id, artist, pictureXl = xl))

    @Test
    fun `names match ignoring case, accents, punctuation and The`() {
        assertEquals("beyonce", matchKey("Beyoncé"))
        assertEquals("beatles", matchKey("The Beatles"))
        assertEquals("simon and garfunkel", matchKey("Simon & Garfunkel"))
        assertEquals("ac dc", matchKey("AC/DC"))
        assertEquals("sigur ros", matchKey("  Sigur Rós "))
        assertEquals("the", matchKey("The The"))
        assertEquals("坂本龍一", matchKey("坂本龍一"))
    }

    @Test
    fun `titles match with edition notes but not by accident`() {
        assertTrue(sameTitle("underdogs", "Underdogs"))
        assertTrue(sameTitle("underdogs", "underdogs (Deluxe Edition)"))
        assertTrue(sameTitle("Call Me Nihilist [Remastered]", "Call Me Nihilist"))
        assertFalse(sameTitle("underdogs", "underdog"))
        assertFalse(sameTitle("Go", "Go Away"))
        assertFalse(sameTitle("", "anything"))
    }

    @Test
    fun `an artist counts only with one of our titles`() {
        // Two artists called Sally; ours made "underdogs".
        val hits = listOf(
            hit("Pop Hits", 1, "Sally", "https://x/famous.jpg"),
            hit("Underdogs", 2, "sally", "https://x/ours.jpg"),
            hit("Underdogs", 3, "Sally Tribute Band", "https://x/tribute.jpg"),
        )
        assertEquals(2L, verifiedArtist("Sally", "underdogs", hits)!!.id)
        assertNull(verifiedArtist("Sally", "Call Me Nihilist", hits))
        assertNull(verifiedArtist("", "underdogs", hits))
    }

    @Test
    fun `Deezer's placeholder is no picture`() {
        assertNull(pictureOf(DeezerArtist(1, "X", pictureXl = "https://e-cdns-images.dzcdn.net/images/artist//1000x1000-000000-80-0-0.jpg")))
        val real = "https://e-cdns-images.dzcdn.net/images/artist/f2bc007e9133c946ac3c3907ddc5d2ea/1000x1000-000000-80-0-0.jpg"
        assertEquals(real, pictureOf(DeezerArtist(2, "Y", pictureXl = real)))
    }

    @Test
    fun `nobody in particular isn't looked up`() {
        assertFalse(worthLookingUp("Various Artists"))
        assertFalse(worthLookingUp("VA"))
        assertFalse(worthLookingUp("  "))
        assertTrue(worthLookingUp("Various Cruelties"))
        assertTrue(worthLookingUp("System Of A Down"))
    }

    @Test
    fun `the search asks for an album of the artist's and takes the one who made it`() = runBlocking {
        val asked = CopyOnWriteArrayList<String>()
        val server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0).apply {
            createContext("/search/album") { exchange ->
                asked += URLDecoder.decode(exchange.requestURI.rawQuery, "UTF-8")
                val body = """{"data":[
                    {"title":"Pop Hits","artist":{"id":1,"name":"Sally","picture_xl":"https://x/famous.jpg"}},
                    {"title":"Underdogs (Deluxe)","artist":{"id":2,"name":"Sally","picture_xl":"https://x/ours.jpg"}}
                ]}""".toByteArray()
                exchange.sendResponseHeaders(200, body.size.toLong())
                exchange.responseBody.use { it.write(body) }
            }
            start()
        }
        try {
            val deezer = Deezer("http://127.0.0.1:${server.address.port}")
            assertEquals("https://x/ours.jpg", deezer.find("Sally", ArtistClues(albums = listOf("Underdogs"), tracks = emptyList())))
            assertEquals(listOf("""q=artist:"Sally" album:"Underdogs"&limit=15"""), asked)
        } finally {
            server.stop(0)
        }
    }
}
