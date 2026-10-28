package io.github.norbertque.pixeltone.remote

import java.net.HttpURLConnection
import java.net.URL
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test

class PhoneFileServerTest {
    private val bytes = ByteArray(100_000) { (it % 251).toByte() }
    private val server = PhoneFileServer(
        object : PhoneFiles {
            override fun track(trackId: Long) =
                if (trackId == 42L) ServedFile(bytes.size.toLong(), "audio/flac") { bytes.inputStream() } else null

            override fun cover(albumId: Long) = if (albumId == 7L) byteArrayOf(1, 2, 3) else null
        },
    )

    @Before
    fun start() = server.start()

    @After
    fun stop() = server.stop()

    private fun get(url: String, range: String? = null): Pair<Int, ByteArray> {
        val connection = URL(url).openConnection() as HttpURLConnection
        range?.let { connection.setRequestProperty("Range", it) }
        val code = connection.responseCode
        val body = (if (code < 400) connection.inputStream else connection.errorStream)?.use { it.readBytes() } ?: ByteArray(0)
        return code to body
    }

    @Test
    fun `serves the whole file and ranges of it`() {
        val url = server.trackUrl("127.0.0.1", 42)
        val (code, body) = get(url)
        assertEquals(200, code)
        assertArrayEquals(bytes, body)

        val (partial, middle) = get(url, "bytes=1000-1999")
        assertEquals(206, partial)
        assertArrayEquals(bytes.copyOfRange(1000, 2000), middle)

        val (open, rest) = get(url, "bytes=99000-")
        assertEquals(206, open)
        assertArrayEquals(bytes.copyOfRange(99000, 100_000), rest)

        assertEquals(416, get(url, "bytes=200000-").first)
    }

    @Test
    fun `needs this run's secret`() {
        val url = server.trackUrl("127.0.0.1", 42)
        assertEquals(403, get(url.substringBefore("?auth=")).first)
        assertEquals(403, get(url.substringBefore("?auth=") + "?auth=guess").first)
        assertEquals(404, get(server.trackUrl("127.0.0.1", 43)).first)
        assertArrayEquals(byteArrayOf(1, 2, 3), get(server.coverUrl("127.0.0.1", 7)).second)
    }

    @Test
    fun `knows its own addresses`() {
        assertEquals(42L, server.trackIdIn(server.trackUrl("192.168.1.5", 42)))
        assertNull(server.trackIdIn("http://192.168.1.5:9000/pc/file/42?auth=someone-else"))
        assertNull(server.trackIdIn("C:\\Music\\a.flac"))
    }

    @Test
    fun `reads ranges`() {
        assertEquals(0L to 99L, PhoneFileServer.byteRange(null, 100))
        assertEquals(10L to 99L, PhoneFileServer.byteRange("bytes=10-", 100))
        assertEquals(10L to 19L, PhoneFileServer.byteRange("bytes=10-19", 100))
        assertEquals(90L to 99L, PhoneFileServer.byteRange("bytes=-10", 100))
        assertEquals(10L to 99L, PhoneFileServer.byteRange("bytes=10-500", 100))
        assertNull(PhoneFileServer.byteRange("bytes=100-", 100))
        assertNull(PhoneFileServer.byteRange("bytes=1-2,5-6", 100))
        assertNull(PhoneFileServer.byteRange("items=1-2", 100))
    }
}
