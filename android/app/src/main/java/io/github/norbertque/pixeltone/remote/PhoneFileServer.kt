package io.github.norbertque.pixeltone.remote

import java.io.BufferedReader
import java.io.InputStream
import java.io.InputStreamReader
import java.io.OutputStream
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.net.URLDecoder
import java.security.SecureRandom
import java.util.Base64
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

/** A phone track's file as the server hands it out. */
class ServedFile(val size: Long, val mimeType: String, val open: () -> InputStream)

/** Where the server finds what it serves: track files by MediaStore id, album covers as JPEG. */
interface PhoneFiles {
    fun track(trackId: Long): ServedFile?
    fun cover(albumId: Long): ByteArray?
}

/**
 * Serves phone tracks to a PC during a hand-over (`docs/remote-protocol.md`: the original file
 * bytes, with Range for seeking). Every address carries this run's secret, so nothing else on
 * the network can read the phone's music through it.
 */
class PhoneFileServer(private val files: PhoneFiles) {
    private val secret: String = ByteArray(24).also { SecureRandom().nextBytes(it) }
        .let { Base64.getUrlEncoder().withoutPadding().encodeToString(it) }
    private var listener: ServerSocket? = null
    private var workers: ExecutorService? = null

    val port: Int get() = listener?.localPort ?: 0

    @Synchronized
    fun start() {
        if (listener != null) return
        val socket = ServerSocket()
        socket.reuseAddress = true
        socket.bind(InetSocketAddress(0))
        val pool = Executors.newCachedThreadPool { task -> Thread(task, "pixeltone-files").apply { isDaemon = true } }
        listener = socket
        workers = pool
        pool.execute {
            while (!socket.isClosed) {
                val client = runCatching { socket.accept() }.getOrNull() ?: break
                pool.execute { serve(client) }
            }
        }
    }

    @Synchronized
    fun stop() {
        runCatching { listener?.close() }
        workers?.shutdownNow()
        listener = null
        workers = null
    }

    fun trackUrl(host: String, trackId: Long): String = "http://${bracketed(host)}:$port/pc/file/$trackId?auth=$secret"

    fun coverUrl(host: String, albumId: Long): String = "http://${bracketed(host)}:$port/pc/cover/$albumId?auth=$secret"

    /** The track in an address this server handed out in this run, or null for anyone else's. */
    fun trackIdIn(url: String): Long? {
        val match = TRACK_URL.find(url) ?: return null
        return match.groupValues[1].toLongOrNull()?.takeIf { match.groupValues[2] == secret }
    }

    private fun serve(client: Socket) {
        client.use { socket ->
            socket.soTimeout = 30_000
            val reader = BufferedReader(InputStreamReader(socket.getInputStream(), Charsets.ISO_8859_1))
            val requestLine = reader.readLine() ?: return
            val headers = generateSequence { reader.readLine()?.takeIf { it.isNotEmpty() } }
                .mapNotNull { line -> line.indexOf(':').takeIf { it > 0 }?.let { line.substring(0, it).trim().lowercase() to line.substring(it + 1).trim() } }
                .toMap()
            val parts = requestLine.split(' ')
            val out = socket.getOutputStream()
            if (parts.size < 2 || (parts[0] != "GET" && parts[0] != "HEAD")) return status(out, 405, "Method Not Allowed")
            val head = parts[0] == "HEAD"
            val (path, query) = parts[1].split('?', limit = 2).let { it[0] to it.getOrElse(1) { "" } }
            val auth = query.split('&').firstOrNull { it.startsWith("auth=") }?.removePrefix("auth=")
                ?.let { URLDecoder.decode(it, "UTF-8") }
            if (auth != secret) return status(out, 403, "Forbidden")
            val segments = path.trim('/').split('/')
            val id = segments.getOrNull(2)?.toLongOrNull()
            when {
                segments.size == 3 && segments[0] == "pc" && segments[1] == "file" && id != null ->
                    file(out, files.track(id) ?: return status(out, 404, "Not Found"), headers["range"], head)
                segments.size == 3 && segments[0] == "pc" && segments[1] == "cover" && id != null -> {
                    val jpeg = files.cover(id) ?: return status(out, 404, "Not Found")
                    write(out, 200, "OK", listOf("Content-Type" to "image/jpeg", "Content-Length" to "${jpeg.size}"))
                    if (!head) out.write(jpeg)
                }
                else -> status(out, 404, "Not Found")
            }
            out.flush()
        }
    }

    private fun file(out: OutputStream, file: ServedFile, range: String?, head: Boolean) {
        val (from, to) = byteRange(range, file.size) ?: return write(
            out,
            416,
            "Range Not Satisfiable",
            listOf("Content-Range" to "bytes */${file.size}"),
        )
        val length = to - from + 1
        val partial = range != null
        val headers = mutableListOf(
            "Content-Type" to file.mimeType,
            "Content-Length" to "$length",
            "Accept-Ranges" to "bytes",
        )
        if (partial) headers += "Content-Range" to "bytes $from-$to/${file.size}"
        write(out, if (partial) 206 else 200, if (partial) "Partial Content" else "OK", headers)
        if (head || length <= 0) return
        file.open().use { input ->
            var skipped = 0L
            while (skipped < from) {
                val n = input.skip(from - skipped)
                if (n <= 0) return
                skipped += n
            }
            val buffer = ByteArray(64 * 1024)
            var left = length
            while (left > 0) {
                val read = input.read(buffer, 0, minOf(buffer.size.toLong(), left).toInt())
                if (read < 0) break
                out.write(buffer, 0, read)
                left -= read
            }
        }
    }

    private fun status(out: OutputStream, code: Int, reason: String) = write(out, code, reason, listOf("Content-Length" to "0"))

    private fun write(out: OutputStream, code: Int, reason: String, headers: List<Pair<String, String>>) {
        val text = buildString {
            append("HTTP/1.1 $code $reason\r\n")
            for ((name, value) in headers) append("$name: $value\r\n")
            append("Connection: close\r\n\r\n")
        }
        out.write(text.toByteArray(Charsets.ISO_8859_1))
    }

    companion object {
        private val TRACK_URL = Regex("/pc/file/(\\d+)\\?auth=([A-Za-z0-9_-]+)")

        /** The first and last byte a `Range` header asks for (the whole file without one); null if it can't be served. */
        internal fun byteRange(range: String?, size: Long): Pair<Long, Long>? {
            if (range == null) return 0L to size - 1
            val spec = range.trim().removePrefix("bytes=").takeIf { range.trim().startsWith("bytes=") && !it.contains(',') } ?: return null
            val (start, end) = spec.split('-', limit = 2).let { it[0].trim() to it.getOrElse(1) { "" }.trim() }
            return when {
                start.isEmpty() -> end.toLongOrNull()?.takeIf { it > 0 }?.let { maxOf(0, size - it) to size - 1 }
                else -> {
                    val from = start.toLongOrNull() ?: return null
                    val to = end.toLongOrNull()?.coerceAtMost(size - 1) ?: (size - 1)
                    (from to to).takeIf { from in 0 until size && to >= from }
                }
            }
        }

        private fun bracketed(host: String) = if (host.contains(':') && !host.startsWith("[")) "[$host]" else host

        /** This phone's address on the network the PC is on (no packet is sent to find it out). */
        fun addressTowards(host: String, port: Int): String? = runCatching {
            DatagramSocket().use { socket ->
                socket.connect(InetAddress.getByName(host), port)
                socket.localAddress?.hostAddress?.substringBefore('%')
            }
        }.getOrNull()
    }
}
