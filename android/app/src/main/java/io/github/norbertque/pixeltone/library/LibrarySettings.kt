package io.github.norbertque.pixeltone.library

import android.content.Context
import androidx.core.content.edit
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.json.Json

/** What the library leaves out, as chosen in Settings; kept in the app's preferences. */
class LibrarySettings(context: Context) {
    private val prefs = context.getSharedPreferences("library", Context.MODE_PRIVATE)
    private val shortest = MutableStateFlow(prefs.getLong(SHORTEST, 0))
    private val hidden = MutableStateFlow(prefs.getStringSet(HIDDEN, emptySet()).orEmpty().toSet())
    private val separators = MutableStateFlow(
        prefs.getString(SEPARATORS, null)?.let { runCatching { Json.decodeFromString(LIST, it) }.getOrNull() } ?: DEFAULT_ARTIST_SEPARATORS,
    )

    /** Files shorter than this aren't music (voice notes, ringtones); 0 keeps everything. */
    val shortestMs: StateFlow<Long> = shortest.asStateFlow()

    /** Folders ([Track.folder]) whose files stay out of the library. */
    val hiddenFolders: StateFlow<Set<String>> = hidden.asStateFlow()

    private val tracksBy = MutableStateFlow(enumValues<TrackSort>().firstOrNull { it.name == prefs.getString(TRACK_SORT, null) } ?: TrackSort.Title)
    private val albumsBy = MutableStateFlow(enumValues<AlbumSort>().firstOrNull { it.name == prefs.getString(ALBUM_SORT, null) } ?: AlbumSort.Title)

    /** The Tracks tab's order. */
    val trackSort: StateFlow<TrackSort> = tracksBy.asStateFlow()

    /** The Albums tab's order. */
    val albumSort: StateFlow<AlbumSort> = albumsBy.asStateFlow()

    fun setTrackSort(sort: TrackSort) {
        prefs.edit { putString(TRACK_SORT, sort.name) }
        tracksBy.value = sort
    }

    fun setAlbumSort(sort: AlbumSort) {
        prefs.edit { putString(ALBUM_SORT, sort.name) }
        albumsBy.value = sort
    }

    /** What artist tags are split on ([splitArtists]); Windows' by default. */
    val artistSeparators: StateFlow<List<String>> = separators.asStateFlow()

    fun setArtistSeparators(list: List<String>) {
        prefs.edit { putString(SEPARATORS, Json.encodeToString(LIST, list)) }
        separators.value = list
    }

    fun setShortest(ms: Long) {
        prefs.edit { putLong(SHORTEST, ms) }
        shortest.value = ms
    }

    fun setHidden(folder: String, hide: Boolean) {
        val folders = if (hide) hidden.value + folder else hidden.value - folder
        prefs.edit { putStringSet(HIDDEN, folders) }
        hidden.value = folders
    }

    private companion object {
        const val SHORTEST = "shortest_ms"
        const val HIDDEN = "hidden_folders"
        const val SEPARATORS = "artist_separators"
        const val TRACK_SORT = "track_sort"
        const val ALBUM_SORT = "album_sort"
        val LIST = ListSerializer(String.serializer())
    }
}

/** The tracks the library shows: none shorter than [shortestMs], none from a hidden folder. */
fun shownTracks(tracks: List<Track>, shortestMs: Long, hidden: Set<String>): List<Track> =
    tracks.filter { it.durationMs >= shortestMs && it.folder !in hidden }
