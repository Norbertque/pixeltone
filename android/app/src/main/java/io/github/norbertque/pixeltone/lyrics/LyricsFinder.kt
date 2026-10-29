package io.github.norbertque.pixeltone.lyrics

import android.content.Context
import android.net.Uri
import androidx.annotation.OptIn
import androidx.core.content.edit
import androidx.media3.common.MediaItem
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.MetadataRetriever
import androidx.media3.extractor.metadata.id3.BinaryFrame
import androidx.media3.extractor.metadata.id3.TextInformationFrame
import androidx.media3.extractor.metadata.vorbis.VorbisComment
import java.io.File
import java.net.URLEncoder
import java.nio.charset.Charset
import java.security.MessageDigest
import java.util.concurrent.TimeUnit
import kotlin.math.abs
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json
import okhttp3.OkHttpClient
import okhttp3.Request

/** A track's lyrics: synced lines, plain lines, or none at all (instrumental). */
data class Lyrics(val lines: List<LyricLine>, val synced: Boolean, val source: Source, val instrumental: Boolean = false) {
    enum class Source { FILE, LRCLIB }

    companion object {
        fun of(text: String, source: Source): Lyrics? =
            parseLrc(text).let { (lines, synced) -> if (lines.isEmpty()) null else Lyrics(lines, synced, source) }
    }
}

/** Whose lyrics: the file on the phone ([uri], null for a PC's track) and what it is. */
data class LyricsRequest(
    /** Names the track for the cache: its file, or what the PC says about it. */
    val key: String,
    val uri: Uri?,
    val title: String,
    val artist: String?,
    val album: String?,
    val durationMs: Long,
)

@Serializable
private data class Cached(val synced: String? = null, val plain: String? = null, val instrumental: Boolean = false, val fetchedAt: Long)

@Serializable
private data class LrclibTrack(
    val duration: Double? = null,
    val instrumental: Boolean = false,
    @SerialName("plainLyrics") val plain: String? = null,
    @SerialName("syncedLyrics") val synced: String? = null,
)

/**
 * Finds lyrics as on Windows, minus what Android keeps from apps: the file's own tags first
 * (Vorbis comments LYRICS/UNSYNCEDLYRICS, ID3 USLT), then LRCLIB online (when allowed), cached in
 * the app's cache; tracks LRCLIB doesn't know are asked again after a week.
 */
class LyricsFinder(private val context: Context) {
    private val prefs = context.getSharedPreferences("lyrics", Context.MODE_PRIVATE)
    private val json = Json { ignoreUnknownKeys = true }
    private val cache = File(context.cacheDir, "lyrics")
    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .build()
    private val allowed = MutableStateFlow(prefs.getBoolean(ONLINE, true))

    /** Asking LRCLIB is allowed (it gets the track's title, artist, album and length). */
    val online: StateFlow<Boolean> = allowed.asStateFlow()

    fun setOnline(on: Boolean) {
        prefs.edit { putBoolean(ONLINE, on) }
        allowed.value = on
    }

    suspend fun find(request: LyricsRequest): Lyrics? = withContext(Dispatchers.IO) {
        request.uri?.let { runCatching { embedded(it) }.getOrNull() }?.let { return@withContext it }
        val file = File(cache, sha1(request.key) + ".json")
        val cached = runCatching { json.decodeFromString(Cached.serializer(), file.readText()) }.getOrNull()
        val now = System.currentTimeMillis()
        val known = cached != null && (cached.synced != null || cached.plain != null || cached.instrumental)
        if (cached != null && (known || now - cached.fetchedAt < RETRY_MS)) return@withContext cached.toLyrics()
        if (!allowed.value) return@withContext cached?.toLyrics()
        val found = runCatching { lrclib(request) }.getOrElse { return@withContext cached?.toLyrics() }
        val entry = Cached(found?.synced?.takeIf(String::isNotBlank), found?.plain?.takeIf(String::isNotBlank), found?.instrumental == true, now)
        runCatching {
            cache.mkdirs()
            file.writeText(json.encodeToString(Cached.serializer(), entry))
        }
        entry.toLyrics()
    }

    private fun Cached.toLyrics(): Lyrics? = when {
        synced != null -> Lyrics.of(synced, Lyrics.Source.LRCLIB) ?: plain?.let { Lyrics.of(it, Lyrics.Source.LRCLIB) }
        plain != null -> Lyrics.of(plain, Lyrics.Source.LRCLIB)
        instrumental -> Lyrics(emptyList(), synced = false, source = Lyrics.Source.LRCLIB, instrumental = true)
        else -> null
    }

    /** The exact signature first (LRCLIB allows a couple of seconds on the length), then a search. */
    private fun lrclib(request: LyricsRequest): LrclibTrack? {
        val seconds = (request.durationMs + 500) / 1000
        val artist = request.artist ?: return null
        if (!request.album.isNullOrBlank() && seconds > 0) {
            val exact = get(
                "/api/get?track_name=${q(request.title)}&artist_name=${q(artist)}&album_name=${q(request.album)}&duration=$seconds",
            )
            if (exact != null) return json.decodeFromString(LrclibTrack.serializer(), exact)
        }
        val results = get("/api/search?track_name=${q(request.title)}&artist_name=${q(mainArtist(artist))}") ?: return null
        val tracks = json.decodeFromString(ListSerializer(LrclibTrack.serializer()), results)
        return best(tracks, seconds.takeIf { it > 0 })
    }

    private fun get(path: String): String? {
        val request = Request.Builder().url(LRCLIB + path).header("User-Agent", USER_AGENT).build()
        return client.newCall(request).execute().use { resp -> if (resp.isSuccessful) resp.body.string() else null }
    }

    @OptIn(UnstableApi::class)
    private fun embedded(uri: Uri): Lyrics? {
        val groups = MetadataRetriever.Builder(context, MediaItem.fromUri(uri)).build().use {
            it.retrieveTrackGroups().get(10, TimeUnit.SECONDS)
        }
        for (g in 0 until groups.length) {
            val group = groups[g]
            for (f in 0 until group.length) {
                val metadata = group.getFormat(f).metadata ?: continue
                for (i in 0 until metadata.length()) {
                    val text = when (val entry = metadata[i]) {
                        is VorbisComment -> entry.value.takeIf { entry.key.uppercase() in VORBIS_KEYS }
                        is BinaryFrame -> if (entry.id == "USLT") uslt(entry.data) else null
                        is TextInformationFrame ->
                            entry.values.firstOrNull().takeIf { entry.id == "TXXX" && entry.description?.uppercase() in VORBIS_KEYS }
                        else -> null
                    }
                    text?.let { Lyrics.of(it, Lyrics.Source.FILE) }?.let { return it }
                }
            }
        }
        return null
    }

    private companion object {
        const val ONLINE = "online"
        const val LRCLIB = "https://lrclib.net"
        const val USER_AGENT = "Pixeltone for Android (https://github.com/Norbertque/pixeltone)"
        const val RETRY_MS = 7L * 24 * 3600 * 1000
        val VORBIS_KEYS = setOf("LYRICS", "UNSYNCEDLYRICS", "UNSYNCED LYRICS")

        fun q(text: String): String = URLEncoder.encode(text, "UTF-8")

        fun sha1(text: String): String =
            MessageDigest.getInstance("SHA-1").digest(text.toByteArray()).joinToString("") { "%02x".format(it) }
    }
}

/** "Aurora Lane feat. Neon Child" → "Aurora Lane": LRCLIB's search knows the main artist. */
internal fun mainArtist(artist: String): String {
    val lower = artist.lowercase()
    val cut = listOf(" feat.", " feat ", " ft.", " ft ", " featuring ", ";").mapNotNull { d -> lower.indexOf(d).takeIf { it >= 0 } }.minOrNull()
    return (if (cut != null) artist.substring(0, cut) else artist).trim()
}

/** The closest length (within 3 s), synced lyrics first. */
private fun best(results: List<LrclibTrack>, seconds: Long?): LrclibTrack? {
    fun off(t: LrclibTrack) = if (t.duration != null && seconds != null) abs(t.duration - seconds) else 0.0
    return results
        .filter { off(it) <= 3.0 && (it.instrumental || it.synced != null || it.plain != null) }
        .minWithOrNull(compareBy<LrclibTrack> { it.synced == null }.thenBy { off(it) })
}

/** An ID3 USLT frame's text: encoding, language, a description, then the lyrics. */
internal fun uslt(data: ByteArray): String? {
    if (data.size < 5) return null
    val charset: Charset = when (data[0].toInt()) {
        0 -> Charsets.ISO_8859_1
        1 -> Charsets.UTF_16
        2 -> Charsets.UTF_16BE
        else -> Charsets.UTF_8
    }
    val wide = data[0].toInt() == 1 || data[0].toInt() == 2
    // Past the language, skip the description up to its terminator.
    var i = 4
    if (wide) {
        while (i + 1 < data.size && !(data[i].toInt() == 0 && data[i + 1].toInt() == 0)) i += 2
        i += 2
    } else {
        while (i < data.size && data[i].toInt() != 0) i++
        i += 1
    }
    if (i >= data.size) return null
    return String(data, i, data.size - i, charset).trimEnd('\u0000').takeIf(String::isNotBlank)
}
