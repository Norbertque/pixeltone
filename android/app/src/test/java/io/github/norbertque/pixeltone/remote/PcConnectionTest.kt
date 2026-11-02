package io.github.norbertque.pixeltone.remote

import java.io.File
import java.net.ServerSocket
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35])
class PcConnectionTest {
    /** A port nothing listens on: connections to it are refused at once. */
    private val closedPort = ServerSocket(0).use { it.localPort }

    @Test
    fun aPcThatNeverAnswersEndsTheSessionAndSaysSo() = runBlocking {
        val paired = PairedPcs(File.createTempFile("paired-pcs", ".json").apply { delete() })
        val connection = PcConnection(RuntimeEnvironment.getApplication(), paired, firstConnectMs = 1_500)
        val report = async(start = CoroutineStart.UNDISPATCHED) { connection.errors.first() }
        connection.begin(PairedPc("pc", "BERRY-PC", "127.0.0.1", closedPort, "ab".repeat(32), "token"))
        assertFalse(connection.session.value!!.reached)
        assertEquals("BERRY-PC can't be reached. Is it on, with Pixeltone running, on this Wi-Fi?", withTimeout(10_000) { report.await() })
        assertNull(connection.session.value)
    }
}
