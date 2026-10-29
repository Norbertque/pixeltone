package io.github.norbertque.pixeltone.remote

import java.io.ByteArrayOutputStream
import java.io.DataOutputStream
import java.io.File
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.delay
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
 * A jam against Pixeltone's `pp-remote` dev server (`cargo run -p pp-remote --example
 * dev_server`): the owner's phone starts it, a guest joins by its link, adds a track of the PC's
 * library and one of its own, is removed, asks to come back and is let in, and the jam ends.
 * Opt-in with `PIXELTONE_DEV_SERVER=port:code`.
 */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35])
class JamDevServerTest {
    @Test
    fun `a guest joins by the link, adds tracks, and is out when the jam ends`() = runBlocking {
        val setting = System.getenv("PIXELTONE_DEV_SERVER")
        assumeTrue("PIXELTONE_DEV_SERVER not set", setting != null)
        val (port, code) = setting!!.split(':').let { it[0].toInt() to it[1] }
        val context = RuntimeEnvironment.getApplication()
        fun phone(guest: Boolean) =
            PcConnection(context, PairedPcs(File.createTempFile("paired-pcs", ".json").apply { delete() }), guest)

        val owner = phone(guest = false)
        val pc = owner.pair("127.0.0.1", port, code)
        owner.begin(pc)
        withTimeout(10_000) { owner.session.first { it?.connected == true && it.state != null } }
        owner.send(PcCommand.simple("jamStart"))
        val jam = withTimeout(10_000) { owner.session.first { it?.jam != null } }!!.jam!!

        // The link names the PC's address on the network, its certificate and the jam's secret.
        val invite = parseJamLink(jam.link)!!
        assertEquals(pc.fingerprint, invite.fingerprint)
        assertEquals(pc.name, invite.pcName)

        val guest = phone(guest = true)
        val wrong = guest.join(invite.copy(secret = "not-the-secret"), "Petr")
        assertTrue(wrong is JoinOutcome.Refused)
        val joined = guest.join(invite, "Petr") as JoinOutcome.In
        guest.begin(joined.pc)
        val me = withTimeout(10_000) { guest.session.first { it?.guest != null } }!!.guest!!
        assertEquals("Petr", me.name)
        assertTrue(me.permissions.addTracks)
        withTimeout(10_000) { owner.session.first { s -> s?.jam?.guests.orEmpty().any { it.name == "Petr" } } }

        // Guests don't drive playback unless the owner lets them.
        val refused = async(start = CoroutineStart.UNDISPATCHED) { guest.errors.first() }
        guest.send(PcCommand.simple("play"))
        assertTrue(withTimeout(10_000) { refused.await() }.contains("control playback"))

        // A track of the PC's library, and one of the guest's own phone (sent as a copy).
        val found = guest.librarySearch(joined.pc, "A4")
        assertTrue(found.tracks.isNotEmpty())
        guest.send(PcCommand.queueAdd(listOf(found.tracks.first().id)))
        val tone = silence(seconds = 1)
        guest.upload(
            joined.pc,
            UploadSource("tone.wav", tone.size.toLong(), "audio/wav", "From Petr's phone", null, null, 1_000, { tone.inputStream() }),
        ) {}
        val rows = eventually { guest.queue(joined.pc, 0, 100).takeIf { r -> r.count { it.addedBy == "Petr" } == 2 } }
        assertTrue(rows.any { it.title == "From Petr's phone" && it.guestId == joined.guestId })

        // Removed: out at once, and back only with the owner's yes.
        val closed = async(start = CoroutineStart.UNDISPATCHED) { guest.jamClosed.first() }
        owner.send(PcCommand.jamRemoveGuest(joined.guestId))
        assertEquals("removed", withTimeout(10_000) { closed.await() })
        val waiting = guest.join(invite, "Petr") as JoinOutcome.Waiting
        val request = withTimeout(10_000) { owner.session.first { it?.jam?.requests.orEmpty().isNotEmpty() } }!!.jam!!.requests.first()
        assertEquals("Petr", request.name)
        owner.send(PcCommand.jamAnswer(request.id, allow = true))
        val back = eventually { guest.rejoinStatus(invite, waiting.requestId).takeIf { it !is JoinOutcome.Waiting } } as JoinOutcome.In
        guest.begin(back.pc)
        withTimeout(10_000) { guest.session.first { it?.guest != null } }

        // Ended: the guest is out, and the copy left the queue.
        val ended = async(start = CoroutineStart.UNDISPATCHED) { guest.jamClosed.first() }
        owner.send(PcCommand.simple("jamEnd"))
        assertEquals("ended", withTimeout(10_000) { ended.await() })
        eventually { owner.queue(pc, 0, 100).takeIf { r -> r.none { it.title == "From Petr's phone" } } }
        assertTrue(guest.join(invite, "Petr") is JoinOutcome.Refused)
        owner.end()
    }

    private suspend fun <T : Any> eventually(seconds: Int = 15, attempt: suspend () -> T?): T {
        val deadline = System.currentTimeMillis() + seconds * 1000L
        while (true) {
            attempt()?.let { return it }
            check(System.currentTimeMillis() < deadline) { "gave up waiting" }
            delay(100)
        }
    }

    /** A short WAV of silence. */
    private fun silence(seconds: Int): ByteArray {
        val rate = 44_100
        val pcm = ByteArray(rate * seconds * 4)
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
