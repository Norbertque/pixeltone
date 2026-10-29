package io.github.norbertque.pixeltone.library

import java.text.Collator
import java.text.Normalizer
import java.util.Locale

/** One audio file in the phone's library. */
data class Track(
    val id: Long,
    val title: String,
    val artist: String?,
    val albumArtist: String?,
    val album: String?,
    /** MediaStore's album id: where the cover comes from. */
    val albumId: Long,
    val durationMs: Long,
    val discNumber: Int?,
    val trackNumber: Int?,
    val year: Int?,
    /** The file's folder; tells apart albums of the same name that have no album artist. */
    val folder: String,
    val fileName: String = "",
) {
    /** The file, as folder and name: what playlists and favourites remember (MediaStore ids can change). */
    val fileKey: String get() = folder.trimEnd('/') + "/" + fileName
}

data class Album(
    val key: String,
    val title: String,
    /** The album artist, else the artist all its tracks share; null when they differ or are unknown. */
    val artist: String?,
    /** No album artist, and the tracks are by more than one artist. */
    val isCompilation: Boolean,
    val year: Int?,
    val coverId: Long,
    /** In disc and track order. */
    val tracks: List<Track>,
) {
    val durationMs: Long get() = tracks.sumOf { it.durationMs }
}

data class Artist(
    val key: String,
    val name: String,
    /** Albums under their name or with a track of theirs, oldest first. */
    val albums: List<Album>,
    /** Their own tracks, by title. */
    val tracks: List<Track>,
)

data class SearchResult(val tracks: List<Track>, val albums: List<Album>, val artists: List<Artist>)

/** The library arranged for browsing: tracks, albums and artists, sorted for [locale]. */
class LibraryIndex(tracks: List<Track>, locale: Locale = Locale.getDefault()) {
    private val collator = Collator.getInstance(locale).apply { strength = Collator.SECONDARY }

    val tracks: List<Track> = tracks.sortedWith(compareBy(collator, Track::title))
    val albums: List<Album>
    val artists: List<Artist>

    init {
        val albumOrder = compareBy<Track>({ it.discNumber ?: 1 }, { it.trackNumber ?: Int.MAX_VALUE })
            .then(compareBy(collator, Track::title))
        albums = this.tracks.groupBy(::albumKey)
            .mapNotNull { (key, list) -> key?.let { album(it, list.sortedWith(albumOrder)) } }
            .sortedWith(compareBy(collator, Album::title))

        // Artists are told apart by their folded name; the spelling used most often names them.
        val spellings = HashMap<String, HashMap<String, Int>>()
        val ownTracks = HashMap<String, MutableList<Track>>()
        val albumsOf = HashMap<String, LinkedHashSet<Album>>()
        fun note(name: String): String {
            val key = fold(name)
            spellings.getOrPut(key) { HashMap() }.merge(name, 1) { a, b -> a + b }
            return key
        }
        for (track in this.tracks) track.artist?.let { ownTracks.getOrPut(note(it)) { ArrayList() }.add(track) }
        for (album in albums) {
            album.artist?.let { albumsOf.getOrPut(note(it)) { LinkedHashSet() }.add(album) }
            for (track in album.tracks) track.artist?.let { albumsOf.getOrPut(fold(it)) { LinkedHashSet() }.add(album) }
        }
        val oldestFirst = compareBy<Album> { it.year ?: Int.MAX_VALUE }.then(compareBy(collator, Album::title))
        artists = spellings
            .map { (key, names) ->
                Artist(
                    key = key,
                    name = names.maxBy { it.value }.key,
                    albums = albumsOf[key].orEmpty().sortedWith(oldestFirst),
                    tracks = ownTracks[key].orEmpty(),
                )
            }
            .sortedWith(compareBy(collator, Artist::name))
    }

    private val tracksById = this.tracks.associateBy(Track::id)
    private val tracksByFile = this.tracks.associateBy(Track::fileKey)
    private val albumsByKey = albums.associateBy(Album::key)
    private val artistsByKey = artists.associateBy(Artist::key)
    private val trackText = this.tracks.map { fold(listOfNotNull(it.title, it.artist, it.album, it.albumArtist).joinToString(" ")) }
    private val albumText = albums.map { fold(listOfNotNull(it.title, it.artist).joinToString(" ")) }
    private val artistText = artists.map { fold(it.name) }

    fun track(id: Long): Track? = tracksById[id]

    /** The track in the file [Track.fileKey] names, if it's still on the phone. */
    fun trackByFile(key: String): Track? = tracksByFile[key]

    fun album(key: String): Album? = albumsByKey[key]

    fun artist(key: String): Artist? = artistsByKey[key]

    /** The artist written as [name] on a track or album. */
    fun artistNamed(name: String): Artist? = artistsByKey[fold(name)]

    /** Everything whose words contain every word of [query], ignoring case and accents. */
    fun search(query: String): SearchResult {
        val words = fold(query).split(WHITESPACE).filter(String::isNotEmpty)
        if (words.isEmpty()) return SearchResult(tracks, albums, artists)
        fun List<String>.matching() = indices.filter { i -> words.all { it in this[i] } }
        return SearchResult(
            tracks = trackText.matching().map(tracks::get),
            albums = albumText.matching().map(albums::get),
            artists = artistText.matching().map(artists::get),
        )
    }

    private fun album(key: String, tracks: List<Track>): Album {
        val tagged = tracks.firstNotNullOfOrNull { it.albumArtist }
        val trackArtists = tracks.mapNotNull { it.artist }.distinctBy(::fold)
        return Album(
            key = key,
            title = checkNotNull(tracks.first().album),
            artist = tagged ?: trackArtists.singleOrNull(),
            isCompilation = tagged == null && trackArtists.size > 1,
            year = tracks.mapNotNull { it.year }.minOrNull(),
            coverId = tracks.first().albumId,
            tracks = tracks,
        )
    }
}

private val MARKS = Regex("\\p{Mn}+")
private val WHITESPACE = Regex("\\s+")

/** Lower case without accents, for matching: "Příběh" → "pribeh". */
internal fun fold(text: String): String =
    MARKS.replace(Normalizer.normalize(text.trim(), Normalizer.Form.NFD), "").lowercase(Locale.ROOT)

/** Albums are told apart by title and album artist, else by title and folder, as on Windows. */
internal fun albumKey(track: Track): String? {
    val title = track.album ?: return null
    val by = track.albumArtist
    return if (by != null) "a|${fold(title)}|${fold(by)}" else "f|${fold(title)}|${track.folder}"
}
