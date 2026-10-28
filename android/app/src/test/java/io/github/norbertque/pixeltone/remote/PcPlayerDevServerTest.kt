package io.github.norbertque.pixeltone.remote

import android.os.Looper
import androidx.media3.common.Player
import com.sun.net.httpserver.HttpServer
import io.github.norbertque.pixeltone.PixeltoneApp
import java.net.InetSocketAddress
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

/**
 * [PcPlayer] against Pixeltone's `pp-remote` dev server (see [PcConnectionDevServerTest]): the
 * PC's player as a Media3 player. Opt-in with `PIXELTONE_DEV_SERVER=port:code`.
 */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35])
class PcPlayerDevServerTest {
    /** Runs the main looper until [done], for up to [seconds]. */
    private fun waitFor(seconds: Int = 15, done: () -> Boolean) {
        val until = System.currentTimeMillis() + seconds * 1000L
        while (!done()) {
            check(System.currentTimeMillis() < until) { "Timed out" }
            shadowOf(Looper.getMainLooper()).idle()
            Thread.sleep(50)
        }
    }

    @Test
    fun `shows and drives the PC's player`() {
        val setting = System.getenv("PIXELTONE_DEV_SERVER")
        assumeTrue("PIXELTONE_DEV_SERVER not set", setting != null)
        val (port, code) = setting!!.split(':').let { it[0].toInt() to it[1] }

        val app = RuntimeEnvironment.getApplication() as PixeltoneApp
        val remote = app.remote
        val pc = runBlocking { remote.pair("127.0.0.1", port, code) }
        val player = PcPlayer(Looper.getMainLooper(), remote, app.library) { _, _, _ -> }

        // A track the PC fetches from "the phone".
        val track = ByteArray(44 + 44_100 * 4 * 3).also { wavHeader(it, dataBytes = it.size - 44) }
        val files = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0).apply {
            createContext("/pc/") { exchange ->
                exchange.responseHeaders.add("Content-Type", "audio/wav")
                exchange.sendResponseHeaders(200, track.size.toLong())
                exchange.responseBody.use { it.write(track) }
            }
            start()
        }
        try {
            remote.connection.begin(pc)
            waitFor { remote.connection.session.value?.connected == true }
            val url = "http://127.0.0.1:${files.address.port}/pc/file/42?auth=x"
            remote.connection.send(
                PcCommand.load(listOf(HandOverItem(url = url, title = "From the phone", durationMs = 3000)), 0, 0, play = true, repeat = "all"),
            )
            waitFor { player.isPlaying && player.mediaMetadata.title == "From the phone" }
            assertEquals(Player.STATE_READY, player.playbackState)
            assertEquals(1, player.mediaItemCount)
            assertEquals(Player.REPEAT_MODE_ALL, player.repeatMode)
            assertTrue(player.isCommandAvailable(Player.COMMAND_SET_DEVICE_VOLUME_WITH_FLAGS))

            // Commands reach the PC, and its answer comes back as the player's state.
            player.pause()
            waitFor { remote.connection.session.value?.state?.status == "paused" && !player.isPlaying }
            player.repeatMode = Player.REPEAT_MODE_OFF
            waitFor { remote.connection.session.value?.state?.repeat == "off" }
            player.seekTo(1_000)
            waitFor { (remote.connection.session.value?.state?.positionMs ?: 0.0) >= 900.0 }
        } finally {
            remote.connection.end()
            player.release()
            files.stop(0)
            remote.forget(pc)
        }
    }

    /** A WAV header for 44.1 kHz 16-bit stereo silence. */
    private fun wavHeader(into: ByteArray, dataBytes: Int) {
        fun le32(at: Int, v: Int) = (0..3).forEach { into[at + it] = (v shr (8 * it)).toByte() }
        fun le16(at: Int, v: Int) = (0..1).forEach { into[at + it] = (v shr (8 * it)).toByte() }
        "RIFF".toByteArray().copyInto(into, 0)
        le32(4, 36 + dataBytes)
        "WAVEfmt ".toByteArray().copyInto(into, 8)
        le32(16, 16)
        le16(20, 1)
        le16(22, 2)
        le32(24, 44_100)
        le32(28, 44_100 * 4)
        le16(32, 4)
        le16(34, 16)
        "data".toByteArray().copyInto(into, 36)
        le32(40, dataBytes)
    }
}
