package io.github.norbertque.pixeltone.library

import android.content.Context
import androidx.core.content.edit
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** What the library leaves out, as chosen in Settings; kept in the app's preferences. */
class LibrarySettings(context: Context) {
    private val prefs = context.getSharedPreferences("library", Context.MODE_PRIVATE)
    private val shortest = MutableStateFlow(prefs.getLong(SHORTEST, 0))
    private val hidden = MutableStateFlow(prefs.getStringSet(HIDDEN, emptySet()).orEmpty().toSet())

    /** Files shorter than this aren't music (voice notes, ringtones); 0 keeps everything. */
    val shortestMs: StateFlow<Long> = shortest.asStateFlow()

    /** Folders ([Track.folder]) whose files stay out of the library. */
    val hiddenFolders: StateFlow<Set<String>> = hidden.asStateFlow()

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
    }
}

/** The tracks the library shows: none shorter than [shortestMs], none from a hidden folder. */
fun shownTracks(tracks: List<Track>, shortestMs: Long, hidden: Set<String>): List<Track> =
    tracks.filter { it.durationMs >= shortestMs && it.folder !in hidden }
