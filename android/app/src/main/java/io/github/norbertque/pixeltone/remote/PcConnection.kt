package io.github.norbertque.pixeltone.remote

import android.content.Context
import android.os.Build
import android.os.SystemClock
import android.provider.Settings
import java.io.IOException
import java.security.cert.CertificateException
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.builtins.ListSerializer
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener

/** Playback is on this PC; the phone is its remote. */
data class RemoteSession(
    val pc: PairedPc,
    val connected: Boolean = false,
    val state: PcState? = null,
    val volume: PcVolume? = null,
)

class RemoteException(message: String) : IOException(message)

/**
 * The connection to a Pixeltone PC: pairing, and while a session is on, a WebSocket that
 * follows the PC's player (reconnecting after drops) and carries commands to it.
 */
class PcConnection(private val context: Context, private val paired: PairedPcs) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    private val base: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(5, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .writeTimeout(10, TimeUnit.SECONDS)
        .build()

    private val currentSession = MutableStateFlow<RemoteSession?>(null)
    val session: StateFlow<RemoteSession?> = currentSession.asStateFlow()

    private val reports = MutableSharedFlow<String>(extraBufferCapacity = 8, onBufferOverflow = BufferOverflow.DROP_OLDEST)

    /** What went wrong, in words for the user (pairing lost, PC unreachable, refused commands). */
    val errors: SharedFlow<String> = reports.asSharedFlow()

    /** The PC's position at [positionAt] (elapsed realtime), for smooth progress in between. */
    @Volatile private var positionMs: Double = 0.0
    @Volatile private var positionAt: Long = 0L

    /** When the volume last changed here: the PC's echoes of older levels are ignored for a moment. */
    @Volatile private var volumeChangedAt: Long = 0L

    private var socket: WebSocket? = null
    private var generation = 0
    private var reconnectJob: Job? = null
    private val volumeRequests = MutableSharedFlow<Float>(extraBufferCapacity = 1, onBufferOverflow = BufferOverflow.DROP_OLDEST)

    init {
        // A dragged slider sends many values: pass on the latest, a few times a second.
        scope.launch {
            volumeRequests.collect { db ->
                send(PcCommand.setVolume(db))
                delay(VOLUME_INTERVAL_MS)
            }
        }
    }

    /** Where the PC is in its track now, as far as this phone knows. */
    fun positionNow(): Long {
        val state = currentSession.value?.state ?: return 0L
        val elapsed = if (state.isPlaying) SystemClock.elapsedRealtime() - positionAt else 0L
        val duration = state.track?.durationMs?.toLong()?.takeIf { it > 0 } ?: Long.MAX_VALUE
        return (positionMs.toLong() + elapsed).coerceIn(0L, duration)
    }

    /* ------------------------------- Pairing ------------------------------- */

    /** What the PC at this address says about itself (first contact, before pairing). */
    suspend fun info(host: String, port: Int): PcInfo = withContext(Dispatchers.IO) {
        var seen: String? = null
        val client = PcTls.firstContact(base) { seen = it }
        val info = client.newCall(Request.Builder().url(url(host, port, "/v1/info")).build()).execute().use { resp ->
            if (resp.code == 403) throw RemoteException("Pixeltone only answers phones on its own local network.")
            if (!resp.isSuccessful) throw RemoteException("That's not Pixeltone (HTTP ${resp.code}).")
            RemoteJson.decodeFromString(PcInfo.serializer(), resp.body.string())
        }
        if (info.protocol != RemoteProtocol.VERSION) {
            throw RemoteException("Update Pixeltone on this phone and on the PC: they speak different versions.")
        }
        if (seen != info.fingerprint.lowercase()) {
            throw RemoteException("The PC's certificate doesn't match what it says. Try again on your own network.")
        }
        info
    }

    /** Pairs with the code shown on the PC and remembers the PC. */
    suspend fun pair(host: String, port: Int, code: String): PairedPc = withContext(Dispatchers.IO) {
        val info = info(host, port)
        val body = RemoteJson.encodeToString(
            PairRequest.serializer(),
            PairRequest(code = code.filter(Char::isDigit), deviceId = paired.deviceId, deviceName = deviceName()),
        ).toRequestBody(JSON)
        val request = Request.Builder().url(url(host, port, "/v1/pair")).post(body).build()
        val answer = PcTls.pinned(base, info.fingerprint).newCall(request).execute().use { resp ->
            when (resp.code) {
                200 -> RemoteJson.decodeFromString(PairResponse.serializer(), resp.body.string())
                403 -> throw RemoteException("Wrong code. Check the code on the PC.")
                410 -> throw RemoteException("The code expired or was used. Press Pair a phone on the PC again.")
                else -> throw RemoteException("Pairing failed (HTTP ${resp.code}).")
            }
        }
        if (answer.server.fingerprint.lowercase() != info.fingerprint.lowercase()) {
            throw RemoteException("The PC changed while pairing. Try again.")
        }
        PairedPc(
            id = answer.server.id,
            name = answer.server.name,
            host = host,
            port = port,
            fingerprint = info.fingerprint.lowercase(),
            token = answer.token,
        ).also(paired::save)
    }

    /** Unpairs on the PC (if it can be reached) and forgets it here. */
    fun forget(pc: PairedPc) {
        if (currentSession.value?.pc?.id == pc.id) end()
        paired.forget(pc.id)
        scope.launch {
            runCatching {
                val request = authorized(pc, "/v1/pair").delete().build()
                PcTls.pinned(base, pc.fingerprint).newCall(request).execute().close()
            }
        }
    }

    /* ------------------------------- Session ------------------------------- */

    /** Starts following and driving this PC (replacing any other session). */
    @Synchronized
    fun begin(pc: PairedPc) {
        closeSocket()
        positionMs = 0.0
        positionAt = SystemClock.elapsedRealtime()
        currentSession.value = RemoteSession(pc = pc)
        open(pc, ++generation, attempt = 0)
    }

    /** Ends the session: the phone stops following the PC. */
    @Synchronized
    fun end() {
        generation++
        closeSocket()
        currentSession.value = null
    }

    fun send(text: String): Boolean {
        val ws = synchronized(this) { socket } ?: return false
        return ws.send(text)
    }

    fun setVolume(db: Float) {
        volumeChangedAt = SystemClock.elapsedRealtime()
        currentSession.update { s -> s?.copy(volume = s.volume?.copy(db = db, muted = false)) }
        volumeRequests.tryEmit(db)
    }

    /** Where the PC is now, as far as this phone knows (right after a seek, before it says). */
    fun movedTo(ms: Long) {
        positionMs = ms.toDouble()
        positionAt = SystemClock.elapsedRealtime()
    }

    /** A page of the PC's queue. */
    suspend fun queue(pc: PairedPc, offset: Int, limit: Int): List<PcQueueRow> = withContext(Dispatchers.IO) {
        val request = authorized(pc, "/v1/queue?offset=$offset&limit=$limit").build()
        PcTls.pinned(base, pc.fingerprint).newCall(request).execute().use { resp ->
            if (!resp.isSuccessful) throw RemoteException("The PC didn't send its queue (HTTP ${resp.code}).")
            RemoteJson.decodeFromString(ListSerializer(PcQueueRow.serializer()), resp.body.string())
        }
    }

    /** A cover the PC names by [key], as its bytes. */
    fun cover(pc: PairedPc, key: String): ByteArray {
        val request = authorized(pc, "/v1/cover/$key").build()
        return PcTls.pinned(base, pc.fingerprint).newCall(request).execute().use { resp ->
            if (!resp.isSuccessful) throw RemoteException("No cover (HTTP ${resp.code}).")
            resp.body.bytes()
        }
    }

    private fun open(pc: PairedPc, gen: Int, attempt: Int) {
        val client = PcTls.pinned(base, pc.fingerprint).newBuilder()
            .readTimeout(0, TimeUnit.MILLISECONDS)
            .pingInterval(15, TimeUnit.SECONDS)
            .build()
        val request = authorized(pc, "/v1/ws").build()
        val ws = client.newWebSocket(
            request,
            object : WebSocketListener() {
                override fun onOpen(webSocket: WebSocket, response: Response) {
                    if (!current(gen)) return
                    currentSession.update { it?.copy(connected = true) }
                }

                override fun onMessage(webSocket: WebSocket, text: String) {
                    if (!current(gen)) return
                    handle(PcMessage.parse(text) ?: return)
                }

                override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                    webSocket.close(1000, null)
                }

                override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                    dropped(pc, gen, attempt, null)
                }

                override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                    val certificateRejected = generateSequence(t) { it.cause }.any { it is CertificateException }
                    dropped(pc, gen, attempt, response?.code ?: if (certificateRejected) CERT_FAILURE else null)
                }
            },
        )
        synchronized(this) {
            if (current(gen)) socket = ws else ws.cancel()
        }
    }

    private fun dropped(pc: PairedPc, gen: Int, attempt: Int, code: Int?) {
        synchronized(this) {
            if (!current(gen)) return
            socket = null
        }
        when (code) {
            401 -> {
                reports.tryEmit("${pc.name} doesn't know this phone any more. Pair again.")
                paired.forget(pc.id)
                end()
            }
            403 -> {
                reports.tryEmit("${pc.name} only answers phones on its own local network.")
                end()
            }
            CERT_FAILURE -> {
                reports.tryEmit("Couldn't verify ${pc.name}. If Pixeltone was reinstalled there, pair again.")
                end()
            }
            else -> {
                currentSession.update { it?.copy(connected = false) }
                reconnectJob?.cancel()
                reconnectJob = scope.launch {
                    delay(RECONNECT_DELAYS_MS[attempt.coerceAtMost(RECONNECT_DELAYS_MS.lastIndex)])
                    val latest = paired.find(pc.id) ?: pc
                    synchronized(this@PcConnection) {
                        if (current(gen)) open(latest, gen, attempt + 1)
                    }
                }
            }
        }
    }

    private fun handle(message: PcMessage) {
        when (message) {
            is PcMessage.State -> {
                positionMs = message.state.positionMs
                positionAt = SystemClock.elapsedRealtime()
                currentSession.update { it?.copy(state = message.state) }
            }
            is PcMessage.Position -> {
                positionMs = message.positionMs
                positionAt = SystemClock.elapsedRealtime()
            }
            is PcMessage.Volume -> currentSession.update { s ->
                val incoming = message.volume
                val changing = SystemClock.elapsedRealtime() - volumeChangedAt < VOLUME_ECHO_MS
                val local = s?.volume
                s?.copy(
                    volume = if (changing && incoming != null && local != null) {
                        incoming.copy(db = local.db, muted = local.muted)
                    } else {
                        incoming
                    },
                )
            }
            is PcMessage.Error -> reports.tryEmit(message.message)
            is PcMessage.Hello -> {
                // A renamed PC: keep the stored name current.
                val pc = currentSession.value?.pc ?: return
                if (pc.name != message.server.name) {
                    val renamed = pc.copy(name = message.server.name)
                    paired.save(renamed)
                    currentSession.update { it?.copy(pc = renamed) }
                }
            }
            PcMessage.LibraryChanged -> Unit
        }
    }

    @Synchronized
    private fun current(gen: Int): Boolean = gen == generation && currentSession.value != null

    private fun closeSocket() {
        reconnectJob?.cancel()
        reconnectJob = null
        socket?.close(1000, null)
        socket = null
    }

    private fun authorized(pc: PairedPc, path: String): Request.Builder =
        Request.Builder()
            .url(url(pc.host, pc.port, path))
            .header("Authorization", "Bearer ${pc.token}")

    private fun url(host: String, port: Int, path: String): String {
        val h = if (host.contains(':') && !host.startsWith("[")) "[$host]" else host
        return "https://$h:$port$path"
    }

    private fun deviceName(): String =
        runCatching { Settings.Global.getString(context.contentResolver, Settings.Global.DEVICE_NAME) }
            .getOrNull()
            ?.takeIf { it.isNotBlank() }
            ?: Build.MODEL
            ?: "Android"

    private companion object {
        const val VOLUME_INTERVAL_MS = 80L
        const val VOLUME_ECHO_MS = 700L
        const val CERT_FAILURE = -1
        val RECONNECT_DELAYS_MS = longArrayOf(500, 1_000, 2_000, 4_000, 8_000)
        val JSON = "application/json".toMediaType()
    }
}
