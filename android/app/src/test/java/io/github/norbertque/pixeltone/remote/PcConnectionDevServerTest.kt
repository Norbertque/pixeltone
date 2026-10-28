package io.github.norbertque.pixeltone.remote

import com.sun.net.httpserver.HttpServer
import java.io.ByteArrayOutputStream
import java.io.DataOutputStream
import java.io.File
import java.net.InetSocketAddress
import java.util.concurrent.CopyOnWriteArrayList
import kotlin.math.PI
import kotlin.math.sin
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

/**
 * Against a real Pixeltone remote server: Pixeltone's `pp-remote` dev server, started with
 * `cargo run -p pp-remote --example dev_server`. Opt-in: set `PIXELTONE_DEV_SERVER` to
 * `port:code` (the code it prints); skipped otherwise.
 */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35])
class PcConnectionDevServerTest {
    @Test
    fun `pairs, follows and hands a queue over`() = runBlocking {
        val setting = System.getenv("PIXELTONE_DEV_SERVER")
        assumeTrue("PIXELTONE_DEV_SERVER not set", setting != null)
        val (port, code) = setting!!.split(':').let { it[0].toInt() to it[1] }

        val paired = PairedPcs(File.createTempFile("paired-pcs", ".json").apply { delete() })
        val connection = PcConnection(RuntimeEnvironment.getApplication(), paired)

        // First contact: the certificate it shows is the one it names.
        val info = connection.info("127.0.0.1", port)
        assertEquals("Pixeltone", info.app)
        assertEquals(RemoteProtocol.VERSION, info.protocol)

        val wrong = runCatching { connection.pair("127.0.0.1", port, if (code == "000000") "000001" else "000000") }
        assertTrue(wrong.exceptionOrNull()?.message.orEmpty().contains("Wrong code"))

        val pc = connection.pair("127.0.0.1", port, code)
        assertEquals(info.fingerprint, pc.fingerprint)
        assertTrue(pc.token.isNotEmpty())
        assertEquals(listOf(pc), paired.pcs.value)

        // A pinned connection that refuses another certificate.
        connection.begin(pc.copy(fingerprint = "00".repeat(32)))
        withTimeout(10_000) { connection.session.first { it == null } }

        connection.begin(pc)
        // On connect the PC sends hello, state and volume.
        val connected = withTimeout(10_000) {
            connection.session.first { it?.connected == true && it.state != null && it.volume != null }
        }
        assertEquals("app", connected!!.volume?.kind)

        // The phone's file server, as far as the PC can tell.
        val requests = CopyOnWriteArrayList<String>()
        val track = wav(seconds = 2)
        val files = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0).apply {
            createContext("/pc/") { exchange ->
                requests += "${exchange.requestMethod} ${exchange.requestURI}"
                val from = exchange.requestHeaders.getFirst("Range")
                    ?.removePrefix("bytes=")?.substringBefore('-')?.toIntOrNull() ?: 0
                exchange.responseHeaders.add("Content-Type", "audio/wav")
                exchange.responseHeaders.add("Accept-Ranges", "bytes")
                if (from > 0) {
                    exchange.responseHeaders.add("Content-Range", "bytes $from-${track.size - 1}/${track.size}")
                    exchange.sendResponseHeaders(206, (track.size - from).toLong())
                } else {
                    exchange.sendResponseHeaders(200, track.size.toLong())
                }
                exchange.responseBody.use { it.write(track, from, track.size - from) }
            }
            start()
        }
        try {
            val url = "http://127.0.0.1:${files.address.port}/pc/file/42?auth=secret"
            assertTrue(
                connection.send(
                    PcCommand.load(
                        items = listOf(HandOverItem(url = url, title = "From the phone", durationMs = 2000)),
                        start = 0,
                        positionMs = 0,
                        play = true,
                        repeat = "off",
                    ),
                ),
            )
            val playing = withTimeout(15_000) {
                connection.session.first { it?.state?.isPlaying == true && it.state.track?.path == url }
            }
            assertEquals("From the phone", playing!!.state!!.track!!.title)
            assertTrue(requests.first().startsWith("GET /pc/file/42?auth=secret"))
            assertEquals("From the phone", connection.queue(pc, 0, 10).single().title)

            connection.send(PcCommand.simple("pause"))
            withTimeout(10_000) { connection.session.first { it?.state?.status == "paused" } }
            connection.setVolume(-12.5f)
            assertEquals(-12.5f, connection.session.value?.volume?.db)
        } finally {
            connection.end()
            files.stop(0)
        }
        assertEquals(null, connection.session.value)
    }

    /** A 44.1 kHz 16-bit stereo sine as a WAV file. */
    private fun wav(seconds: Int): ByteArray {
        val rate = 44_100
        val frames = rate * seconds
        val data = ByteArrayOutputStream()
        for (i in 0 until frames) {
            val sample = (sin(2 * PI * 440 * i / rate) * 8_000).toInt()
            repeat(2) {
                data.write(sample and 0xff)
                data.write((sample shr 8) and 0xff)
            }
        }
        val pcm = data.toByteArray()
        val out = ByteArrayOutputStream()
        DataOutputStream(out).apply {
            fun le32(v: Int) = writeInt(Integer.reverseBytes(v))
            fun le16(v: Int) = writeShort(java.lang.Short.reverseBytes(v.toShort()).toInt())
            writeBytes("RIFF")
            le32(36 + pcm.size)
            writeBytes("WAVE")
            writeBytes("fmt ")
            le32(16)
            le16(1)
            le16(2)
            le32(rate)
            le32(rate * 4)
            le16(4)
            le16(16)
            writeBytes("data")
            le32(pcm.size)
            write(pcm)
        }
        return out.toByteArray()
    }
}
