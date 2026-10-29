package io.github.norbertque.pixeltone.remote

import android.content.Context
import android.os.Build
import android.os.SystemClock
import android.provider.Settings
import java.io.IOException
import java.io.InputStream
import java.net.URLEncoder
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
import kotlinx.serialization.KSerializer
import kotlinx.serialization.builtins.ListSerializer
import okhttp3.HttpUrl
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MediaType.Companion.toMediaTypeOrNull
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.BufferedSink

/** Playback is on this PC; the phone is its remote (or, as a guest, follows its jam). */
data class RemoteSession(
    val pc: PairedPc,
    val connected: Boolean = false,
    val state: PcState? = null,
    val volume: PcVolume? = null,
    /** The jam that runs on the PC, as its owner sees it. */
    val jam: PcJam? = null,
    /** As a guest: this phone's place in the jam. */
    val guest: GuestView? = null,
)

class RemoteException(message: String) : IOException(message)

/** How asking to join a jam went. */
sealed interface JoinOutcome {
    /** In: [pc] holds the guest's token, for a guest [PcConnection]. */
    data class In(val pc: PairedPc, val guestId: String) : JoinOutcome

    /** This phone was removed from the jam: the owner has to let it back in. */
    data class Waiting(val requestId: String) : JoinOutcome

    data class Refused(val message: String) : JoinOutcome
}

/** A phone track to send to a jam. */
class UploadSource(
    /** The file's name, for its extension. */
    val name: String,
    val size: Long,
    val mimeType: String,
    val title: String?,
    val artist: String?,
    val album: String?,
    val durationMs: Long?,
    val open: () -> InputStream,
)

/**
 * The connection to a Pixeltone PC: pairing, and while a session is on, a WebSocket that
 * follows the PC's player (reconnecting after drops) and carries commands to it. As a jam's
 * guest ([guest]) it holds the jam's token instead of a pairing: nothing is remembered, and
 * the session ends with the guest's access.
 */
class PcConnection(
    private val context: Context,
    private val paired: PairedPcs,
    private val guest: Boolean = false,
) {
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

    private val closed = MutableSharedFlow<String>(extraBufferCapacity = 4, onBufferOverflow = BufferOverflow.DROP_OLDEST)

    /** As a guest: why the jam is over for this phone ("ended", "removed"). */
    val jamClosed: SharedFlow<String> = closed.asSharedFlow()

    /** Uploads take their time: a long track over Wi-Fi. */
    private val uploads: OkHttpClient = base.newBuilder()
        .writeTimeout(60, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .build()

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

    /* --------------------------------- Jam --------------------------------- */

    /** Asks to join the jam of [invite] as [name]. */
    suspend fun join(invite: JamInvite, name: String): JoinOutcome = withContext(Dispatchers.IO) {
        val body = RemoteJson.encodeToString(
            JoinRequest.serializer(),
            JoinRequest(secret = invite.secret, deviceId = paired.deviceId, name = name),
        ).toRequestBody(JSON)
        val request = Request.Builder().url(url(invite.host, invite.port, "/v1/jam/join")).post(body).build()
        joinOutcome(invite, request)
    }

    /** Where a rejoin request stands: [JoinOutcome.Waiting] while the owner hasn't answered. */
    suspend fun rejoinStatus(invite: JamInvite, requestId: String): JoinOutcome = withContext(Dispatchers.IO) {
        val request = Request.Builder().url(url(invite.host, invite.port, "/v1/jam/requests/$requestId")).build()
        joinOutcome(invite, request, asking = true)
    }

    private fun joinOutcome(invite: JamInvite, request: Request, asking: Boolean = false): JoinOutcome {
        val call = PcTls.pinned(base, invite.fingerprint).newCall(request)
        val response = try {
            call.execute()
        } catch (e: IOException) {
            val certificate = generateSequence<Throwable>(e) { it.cause }.any { it is CertificateException }
            return JoinOutcome.Refused(
                if (certificate) {
                    "That isn't the PC the code belongs to. Scan it again on the same Wi-Fi."
                } else {
                    "Can't reach ${invite.pcName}. Is this phone on the same Wi-Fi?"
                },
            )
        }
        return response.use { resp ->
            when (resp.code) {
                200 -> {
                    val answer = RemoteJson.decodeFromString(JoinResponse.serializer(), resp.body.string())
                    if (answer.server.fingerprint.lowercase() != invite.fingerprint) {
                        JoinOutcome.Refused("The PC changed. Scan the code again.")
                    } else {
                        val pc = PairedPc(
                            id = answer.server.id,
                            name = answer.server.name,
                            host = invite.host,
                            port = invite.port,
                            fingerprint = invite.fingerprint,
                            token = answer.token,
                        )
                        JoinOutcome.In(pc, answer.guestId)
                    }
                }
                202 -> JoinOutcome.Waiting(RemoteJson.decodeFromString(JoinWaiting.serializer(), resp.body.string()).requestId)
                403 -> JoinOutcome.Refused("The owner didn't let you back in.")
                409 -> JoinOutcome.Refused("The jam is full.")
                410 if asking -> JoinOutcome.Refused("No one answered in time, or the jam ended. Open its code again to ask again.")
                410 -> JoinOutcome.Refused("This jam has ended.")
                else -> JoinOutcome.Refused("Joining failed (HTTP ${resp.code}).")
            }
        }
    }

    suspend fun librarySearch(pc: PairedPc, query: String): PcLibraryResults = get(
        pc,
        "/v1/library/search?q=${URLEncoder.encode(query, "UTF-8")}&limit=100",
        PcLibraryResults.serializer(),
    )

    suspend fun libraryAlbums(pc: PairedPc, offset: Int, limit: Int): List<PcLibraryAlbum> =
        get(pc, "/v1/library/albums?offset=$offset&limit=$limit", ListSerializer(PcLibraryAlbum.serializer()))

    suspend fun libraryAlbum(pc: PairedPc, id: Long): PcLibraryAlbumDetail =
        get(pc, "/v1/library/albums/$id", PcLibraryAlbumDetail.serializer())

    /** Sends a phone track to the jam (the PC keeps a copy); [progress] gets the bytes sent. */
    suspend fun upload(pc: PairedPc, source: UploadSource, progress: (Long) -> Unit) = withContext(Dispatchers.IO) {
        val target = HttpUrl.Builder()
            .scheme("https")
            .host(pc.host)
            .port(pc.port)
            .addPathSegments("v1/jam/tracks")
            .addQueryParameter("name", source.name)
            .apply {
                source.title?.let { addQueryParameter("title", it) }
                source.artist?.let { addQueryParameter("artist", it) }
                source.album?.let { addQueryParameter("album", it) }
                source.durationMs?.let { addQueryParameter("durationMs", it.toString()) }
            }
            .build()
        val body = object : RequestBody() {
            override fun contentType() = source.mimeType.toMediaTypeOrNull()

            override fun contentLength() = source.size

            override fun writeTo(sink: BufferedSink) {
                source.open().use { input ->
                    val buffer = ByteArray(UPLOAD_CHUNK)
                    var sent = 0L
                    while (true) {
                        val read = input.read(buffer)
                        if (read < 0) break
                        sink.write(buffer, 0, read)
                        sent += read
                        progress(sent)
                    }
                }
            }
        }
        val request = Request.Builder().url(target).header("Authorization", "Bearer ${pc.token}").post(body).build()
        PcTls.pinned(uploads, pc.fingerprint).newCall(request).execute().use { resp ->
            if (!resp.isSuccessful) {
                throw RemoteException(resp.body.string().trim().ifEmpty { "The PC didn't take the track (HTTP ${resp.code})." })
            }
        }
    }

    private suspend fun <T> get(pc: PairedPc, path: String, serializer: KSerializer<T>): T =
        withContext(Dispatchers.IO) {
            PcTls.pinned(base, pc.fingerprint).newCall(authorized(pc, path).build()).execute().use { resp ->
                if (!resp.isSuccessful) throw RemoteException("The PC didn't answer (HTTP ${resp.code}).")
                RemoteJson.decodeFromString(serializer, resp.body.string())
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
            401 if guest -> {
                // The jam ended, or this phone is out of it, while it was away.
                closed.tryEmit("gone")
                end()
            }
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
                    val latest = if (guest) pc else paired.find(pc.id) ?: pc
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
                    if (!guest) paired.save(renamed)
                    currentSession.update { it?.copy(pc = renamed) }
                }
            }
            is PcMessage.Jam -> currentSession.update { it?.copy(jam = message.jam) }
            is PcMessage.Guest -> currentSession.update { it?.copy(guest = message.guest) }
            is PcMessage.JamClosed -> {
                closed.tryEmit(message.reason)
                end()
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
        const val UPLOAD_CHUNK = 64 * 1024
        val RECONNECT_DELAYS_MS = longArrayOf(500, 1_000, 2_000, 4_000, 8_000)
        val JSON = "application/json".toMediaType()
    }
}
