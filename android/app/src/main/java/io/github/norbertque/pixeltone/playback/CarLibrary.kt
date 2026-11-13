package io.github.norbertque.pixeltone.playback

import android.content.ContentResolver
import android.content.Context
import android.net.Uri
import android.os.Bundle
import android.provider.MediaStore
import androidx.annotation.DrawableRes
import androidx.annotation.OptIn
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.MediaConstants
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.Album
import io.github.norbertque.pixeltone.library.AlbumSort
import io.github.norbertque.pixeltone.library.Artist
import io.github.norbertque.pixeltone.library.LibraryIndex
import io.github.norbertque.pixeltone.library.ListedTrack
import io.github.norbertque.pixeltone.library.Listen
import io.github.norbertque.pixeltone.library.Lists
import io.github.norbertque.pixeltone.library.Mix
import io.github.norbertque.pixeltone.library.Mixes
import io.github.norbertque.pixeltone.library.Playlist
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.library.albumArtUri
import io.github.norbertque.pixeltone.library.albumKey
import io.github.norbertque.pixeltone.library.fold
import io.github.norbertque.pixeltone.library.recentlyAdded
import io.github.norbertque.pixeltone.library.recentlyPlayedAlbums
import io.github.norbertque.pixeltone.library.sortAlbums
import java.net.URLDecoder
import java.net.URLEncoder
import kotlin.random.Random

/** What a car asked to play: [tracks] from [start], in order or shuffled. */
data class CarPick(val tracks: List<Track>, val start: Int, val shuffle: Boolean)

/**
 * A spoken request ("Hey Google, play … on Pixeltone"): its words, and what the assistant made
 * of them ([focus] is MediaStore's kind of thing asked for: a track, an album, an artist…).
 */
data class SpokenRequest(
    val query: String,
    val focus: String? = null,
    val title: String? = null,
    val artist: String? = null,
    val album: String? = null,
    val genre: String? = null,
    val playlist: String? = null,
)

/** The spoken request in an item a controller asks to play; null when it isn't one. */
fun spokenRequestOf(item: MediaItem): SpokenRequest? {
    val query = item.requestMetadata.searchQuery ?: return null
    val extras = item.requestMetadata.extras
    return SpokenRequest(
        query = query,
        focus = extras?.getString(MediaStore.EXTRA_MEDIA_FOCUS),
        title = extras?.getString(MediaStore.EXTRA_MEDIA_TITLE),
        artist = extras?.getString(MediaStore.EXTRA_MEDIA_ARTIST),
        album = extras?.getString(MediaStore.EXTRA_MEDIA_ALBUM),
        genre = extras?.getString(MediaStore.EXTRA_MEDIA_GENRE),
        playlist = extras?.getString(MediaStore.EXTRA_MEDIA_PLAYLIST),
    )
}

/** Whether [mediaId] is one of [CarLibrary]'s (rather than a track id of the queue's). */
fun isCarId(mediaId: String): Boolean = mediaId.substringBefore('/') in CAR_KINDS

/**
 * The library as a car shows it (Android Auto, Android Automotive): Home, Playlists, Albums and
 * Artists, as on the phone, down to lists of tracks. An item's id says where it is ("album/…" is
 * an album, "album/…/3-1234" its fourth track), so a track picked in a car plays with the rest
 * of its list, from it.
 */
@OptIn(UnstableApi::class)
class CarLibrary(
    private val context: Context,
    private val library: LibraryIndex,
    private val lists: Lists,
    private val mixes: Mixes?,
    private val listens: List<Listen>,
    private val albumSort: AlbumSort = AlbumSort.Title,
    private val random: Random = Random.Default,
) {
    /** What's in [id]: the tabs, a tab's lists, a list's tracks; null when there's no such thing (any more). */
    fun children(id: String): List<MediaItem>? =
        when (id) {
            ROOT -> listOf(
                // Mixes and albums as covers; Shuffle all as a row.
                tab(HOME, R.string.home, R.drawable.ic_car_home, MediaMetadata.MEDIA_TYPE_FOLDER_MIXED, GRID),
                tab(PLAYLISTS, R.string.tab_playlists, R.drawable.ic_car_playlist, MediaMetadata.MEDIA_TYPE_FOLDER_PLAYLISTS, LIST),
                tab(ALBUMS, R.string.tab_albums, R.drawable.ic_car_album, MediaMetadata.MEDIA_TYPE_FOLDER_ALBUMS, GRID),
                tab(ARTISTS, R.string.tab_artists, R.drawable.ic_car_artist, MediaMetadata.MEDIA_TYPE_FOLDER_ARTISTS, LIST),
            )
            HOME -> home()
            PLAYLISTS -> listOf(favouritesItem()) + lists.playlists.map { playlistItem(it) }
            ALBUMS -> sortAlbums(library.albums, albumSort).map { albumItem(it) }
            ARTISTS -> library.artists.map { artistItem(it) }
            else -> {
                val list = parse(id)?.takeIf { it.item == null }
                when (list?.kind) {
                    null -> null
                    ARTIST -> list.key?.let(library::artist)?.let { artist ->
                        artist.albums.map { albumItem(it, group = text(R.string.tab_albums)) } +
                            trackRows(list.list, artist.tracks, group = text(R.string.tab_tracks))
                    }
                    SEARCH -> list.key?.let(::search)
                    else -> tracksOf(list)?.let { trackRows(list.list, it) }
                }
            }
        }

    /** The item [id] stands for; null when there's no such thing (any more). */
    fun item(id: String): MediaItem? {
        if (id == ROOT) return root(context)
        // Not a list's or a track's id: one of the tabs, or nothing.
        val parsed = parse(id) ?: return children(ROOT)?.firstOrNull { it.mediaId == id }
        return when (parsed.item) {
            SHUFFLE -> tracksOf(parsed)?.takeIf { it.isNotEmpty() }?.let { shuffleItem(parsed.list) }
            null -> when (parsed.kind) {
                ALL -> folder(ALL, text(R.string.shuffle_all), count(library.tracks.size))
                FAVOURITES -> favouritesItem()
                PLAYLIST -> parsed.key?.toLongOrNull()?.let(lists::playlist)?.let { playlistItem(it) }
                MIX -> parsed.key?.let { mixes?.mix(it) }?.let { mixItem(it) }
                ALBUM -> parsed.key?.let(library::album)?.let { albumItem(it) }
                ARTIST -> parsed.key?.let(library::artist)?.let { artistItem(it) }
                else -> parsed.key?.let { folder(id, it) }
            }
            else -> pick(parsed)?.let { trackItem(id, it.tracks[it.start]) }
        }
    }

    /** What [query] finds, for a car's search: tracks first, then albums, artists and playlists, each under its heading. */
    fun search(query: String): List<MediaItem> {
        val words = fold(query).split(' ').filter(String::isNotEmpty)
        if (words.isEmpty()) return emptyList()
        val found = library.search(query)
        val list = "$SEARCH/${encode(query)}"
        val tracks = text(R.string.tab_tracks)
        val albums = text(R.string.tab_albums)
        val artists = text(R.string.tab_artists)
        val playlists = text(R.string.tab_playlists)
        return found.tracks.take(MAX_FOUND_TRACKS).mapIndexed { i, track -> trackItem("$list/$i-${track.id}", track, tracks) } +
            found.albums.take(MAX_FOUND).map { albumItem(it, albums) } +
            found.artists.take(MAX_FOUND).map { artistItem(it, artists) } +
            lists.playlists.filter { p -> words.all { it in fold(p.name) } }.map { playlistItem(it, playlists) }
    }

    /** What playing [id] plays: a list from its start, from one of its tracks, or shuffled; null when it's gone. */
    fun pick(id: String): CarPick? = parse(id)?.let { pick(it) }

    private fun pick(id: CarId): CarPick? {
        val tracks = tracksOf(id)
        return when (val item = id.item) {
            null -> inOrder(tracks)
            SHUFFLE -> shuffled(tracks)
            else -> {
                val position = item.substringBefore('-').toIntOrNull()
                val trackId = item.substringAfter('-').toLongOrNull() ?: return null
                val list = tracks.orEmpty()
                val at = if (position != null && list.getOrNull(position)?.id == trackId) position else list.indexOfFirst { it.id == trackId }
                // The list changed since the car showed it: the track alone, if it's still there.
                if (at >= 0) CarPick(list, at, shuffle = false) else library.track(trackId)?.let { inOrder(listOf(it)) }
            }
        }
    }

    /** The tracks [id] plays when added to the queue: a list's, or the one track. */
    fun tracks(id: String): List<Track> {
        val parsed = parse(id) ?: return emptyList()
        val pick = pick(parsed) ?: return emptyList()
        return if (parsed.item == null || parsed.item == SHUFFLE) pick.tracks else listOf(pick.tracks[pick.start])
    }

    /**
     * What a spoken request plays: what the assistant names (a track, an album, an artist, a
     * genre, a playlist), else what's called as asked, else what the words find; with no words,
     * today's mix. Artists and genres play shuffled, a track goes on with its album.
     */
    fun spoken(request: SpokenRequest): CarPick? {
        when (request.focus) {
            MediaStore.Audio.Media.ENTRY_CONTENT_TYPE -> request.title?.let { title ->
                library.tracks.firstOrNull { same(it.title, title) && (request.artist == null || byArtist(it, request.artist)) }
            }?.let { return fromTrack(it) }
            MediaStore.Audio.Albums.ENTRY_CONTENT_TYPE -> request.album?.let { title ->
                library.albums.firstOrNull { same(it.title, title) && (request.artist == null || it.tracks.any { t -> byArtist(t, request.artist) }) }
            }?.let { return inOrder(it.tracks) }
            MediaStore.Audio.Artists.ENTRY_CONTENT_TYPE -> request.artist?.let(library::artistNamed)?.let { return shuffled(it.tracks) }
            MediaStore.Audio.Genres.ENTRY_CONTENT_TYPE -> request.genre?.let { name -> library.genres.firstOrNull { same(it.name, name) } }
                ?.let { return shuffled(it.tracks) }
            MediaStore.Audio.Playlists.ENTRY_CONTENT_TYPE -> request.playlist?.let(::named)?.let { return it }
        }
        val query = request.query.trim()
        if (query.isEmpty()) return inOrder(mixes?.daily?.tracks) ?: shuffled(library.tracks)
        named(query)?.let { return it }
        library.albums.firstOrNull { same(it.title, query) }?.let { return inOrder(it.tracks) }
        library.artists.firstOrNull { same(it.name, query) }?.let { return shuffled(it.tracks) }
        library.genres.firstOrNull { same(it.name, query) }?.let { return shuffled(it.tracks) }
        library.tracks.firstOrNull { same(it.title, query) }?.let { return fromTrack(it) }
        val found = library.search(query)
        inOrder(found.tracks)?.let { return it }
        found.albums.firstOrNull()?.let { return inOrder(it.tracks) }
        found.artists.firstOrNull()?.let { return shuffled(it.tracks) }
        found.genres.firstOrNull()?.let { return shuffled(it.tracks) }
        // "Get Lucky by Daft Punk"
        val (title, artist) = query.split(" by ", ignoreCase = true, limit = 2).takeIf { it.size == 2 } ?: return null
        val words = fold(title).split(' ').filter(String::isNotEmpty)
        return library.tracks.firstOrNull { track -> byArtist(track, artist) && words.all { it in fold(track.title) } }?.let(::fromTrack)
    }

    /* --------------------------------- Lists --------------------------------- */

    /** An item's id taken apart: its list ("album/…") and the track in it ([position]-[id]) or [SHUFFLE], if any. */
    private class CarId(val kind: String, val key: String?, val item: String?) {
        val list: String get() = if (key == null) kind else "$kind/${encode(key)}"
    }

    private fun parse(id: String): CarId? {
        val parts = id.split('/')
        val keyed = when (parts[0]) {
            MIX, PLAYLIST, ALBUM, ARTIST, SEARCH -> true
            FAVOURITES, ALL -> false
            else -> return null
        }
        val size = if (keyed) 2 else 1
        if (parts.size !in size..size + 1) return null
        val key = if (keyed) decode(parts[1]) ?: return null else null
        return CarId(parts[0], key, parts.getOrNull(size))
    }

    private fun tracksOf(id: CarId): List<Track>? =
        when (id.kind) {
            ALL -> library.tracks
            FAVOURITES -> inLibrary(lists.favourites)
            PLAYLIST -> id.key?.toLongOrNull()?.let(lists::playlist)?.let { inLibrary(it.tracks) }
            MIX -> id.key?.let { mixes?.mix(it) }?.tracks
            ALBUM -> id.key?.let(library::album)?.tracks
            ARTIST -> id.key?.let(library::artist)?.tracks
            SEARCH -> id.key?.let { library.search(it).tracks.take(MAX_FOUND_TRACKS) }
            else -> null
        }

    /** The tracks of one of the user's lists that the library (still) has. */
    private fun inLibrary(listed: List<ListedTrack>): List<Track> = listed.mapNotNull { library.trackByFile(it.file) }

    private fun inOrder(tracks: List<Track>?): CarPick? =
        tracks?.takeIf { it.isNotEmpty() }?.let { CarPick(it, 0, shuffle = false) }

    private fun shuffled(tracks: List<Track>?): CarPick? =
        tracks?.takeIf { it.isNotEmpty() }?.let { CarPick(it, random.nextInt(it.size), shuffle = true) }

    /** [track], and then the rest of its album. */
    private fun fromTrack(track: Track): CarPick {
        val album = albumKey(track)?.let(library::album)?.tracks ?: listOf(track)
        return CarPick(album, album.indexOfFirst { it.id == track.id }.coerceAtLeast(0), shuffle = false)
    }

    /** Favourites, today's mix, a genre's mix or a playlist called [name]. */
    private fun named(name: String): CarPick? {
        if (same(name, text(R.string.favourites)) || same(name, "favorites")) return inOrder(inLibrary(lists.favourites))
        mixes?.all?.firstOrNull { same(mixName(it), name) }?.let { return inOrder(it.tracks) }
        return lists.playlists.firstOrNull { same(it.name, name) }?.let { inOrder(inLibrary(it.tracks)) }
    }

    private fun byArtist(track: Track, artist: String): Boolean =
        library.artistNames(track.artist).any { same(it, artist) } || same(track.albumArtist, artist)

    /* --------------------------------- Items --------------------------------- */

    private fun home(): List<MediaItem> {
        val made = text(R.string.made_for_you)
        val played = recentlyPlayedAlbums(library, listens)
        val shown = played.mapTo(HashSet()) { it.key }
        return buildList {
            if (library.tracks.isNotEmpty()) add(shuffleItem(ALL, title = text(R.string.shuffle_all)))
            mixes?.all.orEmpty().filter { it.tracks.isNotEmpty() }.forEach { add(mixItem(it, made)) }
            played.forEach { add(albumItem(it, text(R.string.recently_played))) }
            // An album once: the car tells its items apart by id.
            recentlyAdded(library).filter { it.key !in shown }.forEach { add(albumItem(it, text(R.string.recently_added))) }
        }
    }

    private fun tab(id: String, title: Int, @DrawableRes icon: Int, type: Int, browsableStyle: Int): MediaItem =
        folder(id, text(title), artwork = icon(icon), type = type, browsableStyle = browsableStyle, playableStyle = LIST)

    private fun favouritesItem(): MediaItem =
        folder(FAVOURITES, text(R.string.favourites), count(lists.favourites.size), icon(R.drawable.ic_car_favourite), MediaMetadata.MEDIA_TYPE_PLAYLIST)

    private fun playlistItem(playlist: Playlist, group: String? = null): MediaItem =
        folder("$PLAYLIST/${playlist.id}", playlist.name, count(playlist.tracks.size), icon(R.drawable.ic_car_playlist), MediaMetadata.MEDIA_TYPE_PLAYLIST, group)

    private fun mixItem(mix: Mix, group: String? = null): MediaItem =
        folder(
            "$MIX/${encode(mix.key)}",
            mixName(mix),
            count(mix.tracks.size),
            mix.tracks.firstOrNull()?.let { albumArtUri(it.albumId) },
            MediaMetadata.MEDIA_TYPE_PLAYLIST,
            group,
        )

    private fun albumItem(album: Album, group: String? = null): MediaItem =
        folder(
            "$ALBUM/${encode(album.key)}",
            album.title,
            album.artist ?: text(if (album.isCompilation) R.string.various_artists else R.string.unknown_artist),
            albumArtUri(album.coverId),
            MediaMetadata.MEDIA_TYPE_ALBUM,
            group,
        )

    /** An artist, with one of their covers as the phone shows them; their albums as covers, their tracks as rows. */
    private fun artistItem(artist: Artist, group: String? = null): MediaItem =
        folder(
            "$ARTIST/${encode(artist.key)}",
            artist.name,
            listOfNotNull(
                plural(R.plurals.albums_count, artist.albums.size).takeIf { artist.albums.isNotEmpty() },
                count(artist.tracks.size).takeIf { artist.tracks.isNotEmpty() },
            ).joinToString(" · "),
            (artist.albums.firstOrNull()?.coverId ?: artist.tracks.firstOrNull()?.albumId)?.let(::albumArtUri),
            MediaMetadata.MEDIA_TYPE_ARTIST,
            group,
            browsableStyle = GRID,
        )

    /** A list's tracks, after a Shuffle when there's more than one. */
    private fun trackRows(list: String, tracks: List<Track>, group: String? = null): List<MediaItem> =
        (if (tracks.size > 1) listOf(shuffleItem(list, group)) else emptyList()) +
            tracks.mapIndexed { i, track -> trackItem("$list/$i-${track.id}", track, group) }

    private fun trackItem(id: String, track: Track, group: String? = null): MediaItem =
        MediaItem.Builder()
            .setMediaId(id)
            .setMediaMetadata(
                MediaMetadata.Builder()
                    .setTitle(track.title)
                    .setDisplayTitle(track.title)
                    .setSubtitle(track.artist ?: text(R.string.unknown_artist))
                    .setArtist(track.artist)
                    .setAlbumTitle(track.album)
                    .setArtworkUri(albumArtUri(track.albumId))
                    .setIsPlayable(true)
                    .setIsBrowsable(false)
                    .setMediaType(MediaMetadata.MEDIA_TYPE_MUSIC)
                    .setExtras(extras(group))
                    .build(),
            )
            .build()

    private fun shuffleItem(list: String, group: String? = null, title: String = text(R.string.shuffle)): MediaItem =
        MediaItem.Builder()
            .setMediaId("$list/$SHUFFLE")
            .setMediaMetadata(
                MediaMetadata.Builder()
                    .setTitle(title)
                    .setDisplayTitle(title)
                    .setArtworkUri(icon(R.drawable.ic_car_shuffle))
                    .setIsPlayable(true)
                    .setIsBrowsable(false)
                    .setExtras(extras(group))
                    .build(),
            )
            .build()

    private fun folder(
        id: String,
        title: String,
        subtitle: String? = null,
        artwork: Uri? = null,
        type: Int? = null,
        group: String? = null,
        browsableStyle: Int? = null,
        playableStyle: Int? = null,
    ): MediaItem =
        MediaItem.Builder()
            .setMediaId(id)
            .setMediaMetadata(
                MediaMetadata.Builder()
                    .setTitle(title)
                    .setDisplayTitle(title)
                    .setSubtitle(subtitle)
                    .setArtworkUri(artwork)
                    .setIsPlayable(false)
                    .setIsBrowsable(true)
                    .setMediaType(type)
                    .setExtras(extras(group, browsableStyle, playableStyle))
                    .build(),
            )
            .build()

    /** How a car shows an item: under a heading ([group]), and its children as rows or covers. */
    private fun extras(group: String? = null, browsableStyle: Int? = null, playableStyle: Int? = null): Bundle? {
        if (group == null && browsableStyle == null && playableStyle == null) return null
        return Bundle().apply {
            group?.let { putString(MediaConstants.EXTRAS_KEY_CONTENT_STYLE_GROUP_TITLE, it) }
            browsableStyle?.let { putInt(MediaConstants.EXTRAS_KEY_CONTENT_STYLE_BROWSABLE, it) }
            playableStyle?.let { putInt(MediaConstants.EXTRAS_KEY_CONTENT_STYLE_PLAYABLE, it) }
        }
    }

    /** One of the app's drawables, as cars load pictures. */
    private fun icon(@DrawableRes id: Int): Uri =
        Uri.Builder()
            .scheme(ContentResolver.SCHEME_ANDROID_RESOURCE)
            .authority(context.packageName)
            .appendPath(context.resources.getResourceTypeName(id))
            .appendPath(context.resources.getResourceEntryName(id))
            .build()

    private fun mixName(mix: Mix): String = mix.genre?.let { context.getString(R.string.genre_mix, it.name) } ?: text(R.string.todays_mix)

    private fun count(tracks: Int): String = plural(R.plurals.tracks_count, tracks)

    private fun plural(id: Int, n: Int): String = context.resources.getQuantityString(id, n, n)

    private fun text(id: Int): String = context.getString(id)

    companion object {
        const val ROOT = "root"

        private const val LIST = MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM
        private const val GRID = MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_GRID_ITEM

        /** The top of the library; its children are the car's tabs. */
        fun root(context: Context): MediaItem =
            MediaItem.Builder()
                .setMediaId(ROOT)
                .setMediaMetadata(
                    MediaMetadata.Builder()
                        .setTitle(context.getString(R.string.library))
                        .setIsPlayable(false)
                        .setIsBrowsable(true)
                        .setMediaType(MediaMetadata.MEDIA_TYPE_FOLDER_MIXED)
                        .build(),
                )
                .build()
    }
}

/** Names that are the same but for case, accents, spaces and punctuation ("Today's mix" = "today’s mix"). */
private fun same(a: String?, b: String?): Boolean {
    if (a == null || b == null) return false
    val key = { text: String -> fold(text).filter(Char::isLetterOrDigit) }
    return key(a).let { it.isNotEmpty() && it == key(b) }
}

private fun encode(key: String): String = URLEncoder.encode(key, Charsets.UTF_8.name())

private fun decode(part: String): String? = runCatching { URLDecoder.decode(part, Charsets.UTF_8.name()) }.getOrNull()

private const val HOME = "home"
private const val PLAYLISTS = "playlists"
private const val ALBUMS = "albums"
private const val ARTISTS = "artists"
private const val ALL = "all"
private const val FAVOURITES = "favourites"
private const val PLAYLIST = "playlist"
private const val MIX = "mix"
private const val ALBUM = "album"
private const val ARTIST = "artist"
private const val SEARCH = "search"
private const val SHUFFLE = "shuffle"
private val CAR_KINDS = setOf(CarLibrary.ROOT, HOME, PLAYLISTS, ALBUMS, ARTISTS, ALL, FAVOURITES, PLAYLIST, MIX, ALBUM, ARTIST, SEARCH)

/** A search shows (and plays) this many of the tracks it finds, and so many albums and artists. */
private const val MAX_FOUND_TRACKS = 50
private const val MAX_FOUND = 20
