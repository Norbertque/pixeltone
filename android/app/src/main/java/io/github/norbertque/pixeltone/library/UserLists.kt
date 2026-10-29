package io.github.norbertque.pixeltone.library

import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

/**
 * A track in a list of the user's: its file ([Track.fileKey]), and what it was, so a track
 * that left the phone can still be named.
 */
@Serializable
data class ListedTrack(
    val file: String,
    val title: String,
    val artist: String? = null,
    val album: String? = null,
) {
    companion object {
        fun of(track: Track) = ListedTrack(track.fileKey, track.title, track.artist, track.album)
    }
}

@Serializable
data class Playlist(val id: Long, val name: String, val tracks: List<ListedTrack> = emptyList())

/** Favourites (newest first) and playlists. */
@Serializable
data class Lists(
    val favourites: List<ListedTrack> = emptyList(),
    val playlists: List<Playlist> = emptyList(),
    val nextId: Long = 1,
) {
    fun isFavourite(track: Track): Boolean = favourites.any { it.file == track.fileKey }

    fun playlist(id: Long): Playlist? = playlists.firstOrNull { it.id == id }
}

/**
 * The user's favourites and playlists, in a JSON file of the app's own; every change is written
 * in the background, one write at a time.
 */
class UserLists(private val file: File) {
    private val writes = CoroutineScope(SupervisorJob() + Dispatchers.IO.limitedParallelism(1))
    private val json = Json { ignoreUnknownKeys = true }
    private val current = MutableStateFlow(load())

    val lists: StateFlow<Lists> = current.asStateFlow()

    fun setFavourite(track: Track, on: Boolean) = change { lists ->
        val others = lists.favourites.filterNot { it.file == track.fileKey }
        lists.copy(favourites = if (on) listOf(ListedTrack.of(track)) + others else others)
    }

    /** A new playlist holding [tracks]; its id. */
    fun create(name: String, tracks: List<Track> = emptyList()): Long {
        var id = 0L
        change { lists ->
            id = lists.nextId
            val playlist = Playlist(id, name.trim(), tracks.map(ListedTrack::of))
            lists.copy(playlists = lists.playlists + playlist, nextId = id + 1)
        }
        return id
    }

    fun rename(id: Long, name: String) = edit(id) { it.copy(name = name.trim()) }

    fun delete(id: Long) = change { lists -> lists.copy(playlists = lists.playlists.filterNot { it.id == id }) }

    fun add(id: Long, tracks: List<Track>) = edit(id) { it.copy(tracks = it.tracks + tracks.map(ListedTrack::of)) }

    fun removeAt(id: Long, index: Int) = edit(id) {
        if (index in it.tracks.indices) it.copy(tracks = it.tracks.toMutableList().apply { removeAt(index) }) else it
    }

    fun move(id: Long, from: Int, to: Int) = edit(id) {
        if (from !in it.tracks.indices || to !in it.tracks.indices) return@edit it
        it.copy(tracks = it.tracks.toMutableList().apply { add(to, removeAt(from)) })
    }

    private fun edit(id: Long, change: (Playlist) -> Playlist) = change { lists ->
        lists.copy(playlists = lists.playlists.map { if (it.id == id) change(it) else it })
    }

    private fun change(change: (Lists) -> Lists) {
        current.update(change)
        val snapshot = current.value
        writes.launch { save(snapshot) }
    }

    private fun load(): Lists =
        runCatching { json.decodeFromString(Lists.serializer(), file.readText()) }.getOrNull() ?: Lists()

    private fun save(lists: Lists) {
        file.parentFile?.mkdirs()
        val partial = File(file.parentFile, file.name + ".part")
        partial.writeText(json.encodeToString(Lists.serializer(), lists))
        partial.renameTo(file)
    }
}
