package io.github.norbertque.pixeltone.library

import android.content.Context
import androidx.core.content.edit
import java.io.File
import java.io.IOException
import java.text.Normalizer
import java.util.Locale
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.MapSerializer
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.json.Json
import okhttp3.HttpUrl.Companion.toHttpUrl
import okhttp3.OkHttpClient
import okhttp3.Request

/*
 * Artist pictures from Deezer's public API (no account or key), as on Windows (`pp-deezer`):
 * a name alone isn't enough, as many artists share one ("Sally") and the best-known of them is
 * rarely the one in someone's library. So an artist is only taken when Deezer lists an album
 * (or else a track) of theirs with a title the library has too. What's sent: the artist's name
 * with up to three album and three track titles.
 */

/** An artist as Deezer's search results show them. */
@Serializable
data class DeezerArtist(
    val id: Long = 0,
    val name: String = "",
    @SerialName("picture_big") val pictureBig: String? = null,
    @SerialName("picture_xl") val pictureXl: String? = null,
)

/** An album or a track in Deezer's search results, with its artist. */
@Serializable
data class DeezerHit(val title: String = "", val artist: DeezerArtist = DeezerArtist())

/** What the library knows an artist by, besides the name: album titles (largest first) and track titles. */
data class ArtistClues(val albums: List<String>, val tracks: List<String>)

private val MARKS = Regex("\\p{M}+")
private val NOT_WORD = Regex("[^\\p{L}\\p{N}]+")

/**
 * How names are compared: lower case, without accents, punctuation or a leading "The", and "&"
 * read as "and" ("Beyoncé" = "Beyonce", "The Beatles" = "Beatles").
 */
fun matchKey(name: String): String {
    val folded = MARKS.replace(Normalizer.normalize(name, Normalizer.Form.NFKD), "").lowercase(Locale.ROOT).replace("&", " and ")
    val words = folded.split(NOT_WORD).filter(String::isNotEmpty)
    return (if (words.size > 1 && words[0] == "the") words.drop(1) else words).joinToString(" ")
}

/** Same title, allowing an edition note on either side ("underdogs" = "underdogs (Deluxe)"). */
fun sameTitle(ours: String, theirs: String): Boolean {
    val a = matchKey(ours)
    val b = matchKey(theirs)
    if (a.isEmpty() || b.isEmpty()) return false
    val (short, long) = if (a.length <= b.length) a to b else b to a
    return short == long || (short.length >= 3 && long.startsWith("$short "))
}

/** The artist of the first hit whose artist is [name] and whose title is [title]. */
fun verifiedArtist(name: String, title: String, hits: List<DeezerHit>): DeezerArtist? {
    val key = matchKey(name)
    if (key.isEmpty()) return null
    return hits.firstOrNull { matchKey(it.artist.name) == key && sameTitle(title, it.title) }?.artist
}

/**
 * The largest picture, unless it's Deezer's placeholder for artists without one (an image URL
 * with an empty id: `…/images/artist//1000x1000-….jpg`).
 */
fun pictureOf(artist: DeezerArtist): String? =
    (artist.pictureXl ?: artist.pictureBig)?.takeIf { it.startsWith("http") && "/artist//" !in it }

/** Whether a name stands for someone in particular ("Various Artists" doesn't). */
fun worthLookingUp(name: String): Boolean {
    val key = matchKey(name)
    return key.isNotEmpty() && key !in NOBODY
}

private val NOBODY = setOf(
    "various artists", "various", "va", "unknown", "unknown artist", "artist", "anonymous", "soundtrack", "original soundtrack",
)

/** Deezer asks to slow down (50 requests in 5 seconds at most). */
class DeezerQuotaException : IOException("Deezer asks to slow down")

/** Deezer's search, a request at a time, at least [SPACING_MS] apart. */
class Deezer(private val base: String = "https://api.deezer.com") {
    private val client = OkHttpClient.Builder().callTimeout(15, TimeUnit.SECONDS).build()
    private val json = Json { ignoreUnknownKeys = true }
    private var lastRequest = 0L

    @Serializable
    private data class Search(val data: List<DeezerHit> = emptyList(), val error: ApiError? = null)

    @Serializable
    private data class ApiError(val code: Int = 0, val message: String = "")

    /** The picture of the artist called [name] who made one of the [clues] titles; null when Deezer has none. */
    suspend fun find(name: String, clues: ArtistClues): String? {
        for ((kind, titles) in listOf("album" to clues.albums, "track" to clues.tracks)) {
            for (title in titles.filter { matchKey(it).isNotEmpty() }.take(TRIES)) {
                val artist = verifiedArtist(name, title, search(kind, name, title))
                // The right artist: a placeholder picture means Deezer has none of them.
                if (artist != null) return pictureOf(artist)
            }
        }
        return null
    }

    private suspend fun search(kind: String, artist: String, title: String): List<DeezerHit> {
        val wait = lastRequest + SPACING_MS - System.currentTimeMillis()
        if (wait > 0) delay(wait)
        lastRequest = System.currentTimeMillis()
        fun quoted(text: String) = text.replace('"', ' ')
        val url = "$base/search/$kind".toHttpUrl().newBuilder()
            .addQueryParameter("q", "artist:\"${quoted(artist)}\" $kind:\"${quoted(title)}\"")
            .addQueryParameter("limit", "15")
            .build()
        val request = Request.Builder().url(url).header("User-Agent", USER_AGENT).build()
        val search = withContext(Dispatchers.IO) {
            client.newCall(request).execute().use { response ->
                if (!response.isSuccessful) throw IOException("Deezer: HTTP ${response.code}")
                json.decodeFromString(Search.serializer(), response.body.string())
            }
        }
        search.error?.let { throw if (it.code == 4) DeezerQuotaException() else IOException("Deezer: ${it.message}") }
        return search.data
    }

    private companion object {
        const val TRIES = 3
        const val SPACING_MS = 150L
        const val USER_AGENT = "Pixeltone for Android (https://github.com/Norbertque/pixeltone)"
    }
}

/** What's known of an artist's picture: its address, or none (looked for at [checkedAt], asked again a month on). */
@Serializable
data class ArtistPicture(val url: String? = null, val checkedAt: Long = 0)

/**
 * The library's artist pictures, found in the background as on Windows (`artist_images.rs`):
 * the artists with the most tracks first, a few lookups a second, misses asked again after 30
 * days; offline, the next round goes on where this one stopped. Kept in the app's files;
 * turning them off removes them.
 */
class ArtistPictures(context: Context, private val index: StateFlow<LibraryIndex?>, private val deezer: Deezer = Deezer()) {
    private val prefs = context.getSharedPreferences("artist_pictures", Context.MODE_PRIVATE)
    private val file = File(context.filesDir, "artist-pictures.json")
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val json = Json { ignoreUnknownKeys = true }
    private val serializer = MapSerializer(String.serializer(), ArtistPicture.serializer())
    private val on = MutableStateFlow(prefs.getBoolean(ENABLED, true))
    private val known = MutableStateFlow(load())
    private var started = false

    /** Looking for pictures online is allowed (it sends artist names and a few titles to Deezer). */
    val enabled: StateFlow<Boolean> = on.asStateFlow()

    /** Artist keys ([Artist.key]) with their picture's address. */
    val pictures: StateFlow<Map<String, ArtistPicture>> = known.asStateFlow()

    fun setEnabled(enabled: Boolean) {
        prefs.edit { putBoolean(ENABLED, enabled) }
        on.value = enabled
        if (!enabled) {
            known.value = emptyMap()
            scope.launch { file.delete() }
        }
    }

    /** Starts looking whenever the library changes (and the pictures are on). */
    fun start() {
        if (started) return
        started = true
        scope.launch {
            combine(index.filterNotNull(), on) { library, allowed -> library to allowed }.collectLatest { (library, allowed) ->
                if (allowed) round(library)
            }
        }
    }

    private suspend fun round(library: LibraryIndex) {
        val artists = library.artists.filter { worthLookingUp(it.name) }.sortedByDescending { it.tracks.size }
        for (artist in artists) {
            val now = System.currentTimeMillis()
            val seen = known.value[artist.key]
            if (seen != null && (seen.url != null || now - seen.checkedAt < RETRY_MISSES_MS)) continue
            val clues = ArtistClues(
                albums = artist.albums.sortedByDescending { album -> album.tracks.count { it in artist.tracks } }.map { it.title },
                tracks = artist.tracks.map { it.title },
            )
            val url = try {
                deezer.find(artist.name, clues)
            } catch (_: DeezerQuotaException) {
                delay(QUOTA_PAUSE_MS)
                continue
            } catch (_: IOException) {
                // Offline, or Deezer is away: the next round goes on from here.
                return
            } catch (_: kotlinx.serialization.SerializationException) {
                return
            }
            known.value = known.value + (artist.key to ArtistPicture(url, now))
            save()
            delay(PAUSE_MS)
        }
    }

    private fun load(): Map<String, ArtistPicture> =
        runCatching { json.decodeFromString(serializer, file.readText()) }.getOrDefault(emptyMap())

    private fun save() {
        runCatching { file.writeText(json.encodeToString(serializer, known.value)) }
    }

    private companion object {
        const val ENABLED = "enabled"
        const val PAUSE_MS = 250L
        const val QUOTA_PAUSE_MS = 10_000L
        const val RETRY_MISSES_MS = 30L * 86_400_000
    }
}
