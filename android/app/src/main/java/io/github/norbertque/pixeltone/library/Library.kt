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
    /** When the file came onto the phone (epoch seconds; MediaStore's DATE_ADDED). */
    val addedAt: Long = 0,
    /** Its genres, split as on Windows ([splitGenres]). */
    val genres: List<String> = emptyList(),
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

data class Genre(
    val key: String,
    val name: String,
    /** By title. */
    val tracks: List<Track>,
    /** The albums its tracks are on, by title. */
    val albums: List<Album>,
)

data class SearchResult(
    val tracks: List<Track>,
    val albums: List<Album>,
    val artists: List<Artist>,
    val genres: List<Genre> = emptyList(),
)

/**
 * The library arranged for browsing: tracks, albums, artists and genres, sorted for [locale].
 * Artist tags are split on [separators] ("A feat. B" is A and B).
 */
class LibraryIndex(
    tracks: List<Track>,
    locale: Locale = Locale.getDefault(),
    private val separators: List<String> = DEFAULT_ARTIST_SEPARATORS,
) {
    private val collator = Collator.getInstance(locale).apply { strength = Collator.SECONDARY }
    private val splits = HashMap<String, List<String>>()

    val tracks: List<Track> = tracks.sortedWith(compareBy(collator, Track::title))
    val albums: List<Album>
    val artists: List<Artist>
    val genres: List<Genre>

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
        for (track in this.tracks) {
            for (name in artistNames(track.artist)) ownTracks.getOrPut(note(name)) { ArrayList() }.add(track)
        }
        for (album in albums) {
            for (name in artistNames(album.artist)) albumsOf.getOrPut(note(name)) { LinkedHashSet() }.add(album)
            for (track in album.tracks) {
                for (name in artistNames(track.artist)) albumsOf.getOrPut(fold(name)) { LinkedHashSet() }.add(album)
            }
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

        // Genres too are told apart by their folded name and named by the spelling used most.
        val genreSpellings = HashMap<String, HashMap<String, Int>>()
        val genreTracks = HashMap<String, MutableList<Track>>()
        for (track in this.tracks) {
            for (name in track.genres) {
                val key = fold(name)
                genreSpellings.getOrPut(key) { HashMap() }.merge(name, 1) { a, b -> a + b }
                genreTracks.getOrPut(key) { ArrayList() }.add(track)
            }
        }
        val albumOf = HashMap<Long, Album>()
        for (album in albums) for (track in album.tracks) albumOf[track.id] = album
        genres = genreTracks
            .map { (key, list) ->
                Genre(
                    key = key,
                    name = genreSpellings.getValue(key).maxBy { it.value }.key,
                    tracks = list,
                    albums = list.mapNotNull { albumOf[it.id] }.distinctBy(Album::key).sortedWith(compareBy(collator, Album::title)),
                )
            }
            .sortedWith(compareBy(collator, Genre::name))
    }

    private val tracksById = this.tracks.associateBy(Track::id)
    private val tracksByFile = this.tracks.associateBy(Track::fileKey)
    private val albumsByKey = albums.associateBy(Album::key)
    private val artistsByKey = artists.associateBy(Artist::key)
    private val genresByKey = genres.associateBy(Genre::key)
    private val trackText = this.tracks.map { fold(listOfNotNull(it.title, it.artist, it.album, it.albumArtist).joinToString(" ")) }
    private val albumText = albums.map { fold(listOfNotNull(it.title, it.artist).joinToString(" ")) }
    private val artistText = artists.map { fold(it.name) }
    private val genreText = genres.map(Genre::key)

    fun track(id: Long): Track? = tracksById[id]

    /** The track in the file [Track.fileKey] names, if it's still on the phone. */
    fun trackByFile(key: String): Track? = tracksByFile[key]

    fun album(key: String): Album? = albumsByKey[key]

    fun artist(key: String): Artist? = artistsByKey[key]

    /** The artist written as [name] on a track or album: the first of them when it names several. */
    fun artistNamed(name: String): Artist? = artistsByKey[fold(name)] ?: artistNames(name).firstNotNullOfOrNull { artistsByKey[fold(it)] }

    /** The artists an artist tag names ([splitArtists] on this library's separators). */
    fun artistNames(tag: String?): List<String> =
        if (tag == null) emptyList() else synchronized(splits) { splits.getOrPut(tag) { splitArtists(tag, separators) } }

    fun genre(key: String): Genre? = genresByKey[key]

    /** Everything whose words contain every word of [query], ignoring case and accents. */
    fun search(query: String): SearchResult {
        val words = fold(query).split(WHITESPACE).filter(String::isNotEmpty)
        if (words.isEmpty()) return SearchResult(tracks, albums, artists, genres)
        fun List<String>.matching() = indices.filter { i -> words.all { it in this[i] } }
        return SearchResult(
            tracks = trackText.matching().map(tracks::get),
            albums = albumText.matching().map(albums::get),
            artists = artistText.matching().map(artists::get),
            genres = genreText.matching().map(genres::get),
        )
    }

    private fun album(key: String, tracks: List<Track>): Album {
        val tagged = tracks.firstNotNullOfOrNull { it.albumArtist }
        // "A" and "A feat. B" on one album are still A's album.
        val trackArtists = tracks.mapNotNull { artistNames(it.artist).firstOrNull() }.distinctBy(::fold)
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

private val GENRE_SEPARATORS = Regex("[;/,\\\\\u0000]")
private val ID3_NUMBER = Regex("^\\(\\d+\\)")

/**
 * Genres from the tags, split on the usual separators as on Windows (`split_genres`): "Rock;
 * Pop/Indie" is Rock, Pop and Indie, each once. An old ID3 number ("(17)") names no genre.
 */
fun splitGenres(values: List<String>): List<String> {
    val seen = HashSet<String>()
    return values
        .flatMap { it.split(GENRE_SEPARATORS) }
        .map { ID3_NUMBER.replace(it.trim(), "").trim() }
        .filter { it.isNotEmpty() && !it.all(Char::isDigit) && seen.add(fold(it)) }
}

/** Albums are told apart by title and album artist, else by title and folder, as on Windows. */
internal fun albumKey(track: Track): String? {
    val title = track.album ?: return null
    val by = track.albumArtist
    return if (by != null) "a|${fold(title)}|${fold(by)}" else "f|${fold(title)}|${track.folder}"
}
