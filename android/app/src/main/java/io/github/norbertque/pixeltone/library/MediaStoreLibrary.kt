package io.github.norbertque.pixeltone.library

import android.content.ContentUris
import android.content.Context
import android.database.ContentObserver
import android.database.Cursor
import android.database.sqlite.SQLiteException
import android.net.Uri
import android.os.Build
import android.provider.MediaStore
import androidx.annotation.MainThread
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

/**
 * The phone's music as MediaStore knows it, read again whenever MediaStore changes, less what
 * [settings] leave out.
 */
class MediaStoreLibrary(private val context: Context, settings: LibrarySettings) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val changes = Channel<Unit>(Channel.CONFLATED)
    private val read = MutableStateFlow<List<Track>?>(null)
    private var started = false

    /** Every audio file MediaStore lists as music, left-out ones too; null until the first read is done. */
    val allTracks: StateFlow<List<Track>?> = read.asStateFlow()

    /** Null until the first read is done. */
    val index: StateFlow<LibraryIndex?> =
        combine(read, settings.shortestMs, settings.hiddenFolders, settings.artistSeparators) { tracks, shortest, hidden, separators ->
            tracks?.let { LibraryIndex(shownTracks(it, shortest, hidden), separators = separators) }
        }
            .flowOn(Dispatchers.Default)
            .stateIn(scope, SharingStarted.Eagerly, null)

    /** Reads MediaStore again (after files were added without MediaStore saying so). */
    fun refresh() {
        changes.trySend(Unit)
    }

    /** Starts reading; call once the app may read audio files. */
    @MainThread
    fun start() {
        if (started) return
        started = true
        context.contentResolver.registerContentObserver(
            audioCollection(),
            true,
            object : ContentObserver(null) {
                override fun onChange(selfChange: Boolean) {
                    changes.trySend(Unit)
                }
            },
        )
        changes.trySend(Unit)
        scope.launch {
            for (change in changes) {
                read.value = readTracks()
                // MediaStore reports changes in bursts while it scans: read at most once a second.
                delay(1_000)
            }
        }
    }

    private fun readTracks(): List<Track> {
        val tracks = try {
            read(withAlbumArtist = true)
        } catch (_: IllegalArgumentException) {
            read(withAlbumArtist = false)
        } catch (_: SQLiteException) {
            read(withAlbumArtist = false)
        }
        if (Build.VERSION.SDK_INT >= 30) return tracks
        val genres = olderGenres()
        return tracks.map { track -> genres[track.id]?.let { track.copy(genres = splitGenres(it)) } ?: track }
    }

    /** Before Android 11, genres are tables of their own: each genre and its tracks. */
    @Suppress("DEPRECATION")
    private fun olderGenres(): Map<Long, List<String>> {
        val byTrack = HashMap<Long, MutableList<String>>()
        val resolver = context.contentResolver
        runCatching {
            val genres = MediaStore.Audio.Genres.EXTERNAL_CONTENT_URI
            resolver.query(genres, arrayOf(MediaStore.Audio.Genres._ID, MediaStore.Audio.Genres.NAME), null, null, null)?.use { g ->
                while (g.moveToNext()) {
                    val name = g.text(1) ?: continue
                    val members = MediaStore.Audio.Genres.Members.getContentUri("external", g.getLong(0))
                    resolver.query(members, arrayOf(MediaStore.Audio.Genres.Members.AUDIO_ID), null, null, null)?.use { m ->
                        while (m.moveToNext()) byTrack.getOrPut(m.getLong(0)) { ArrayList() }.add(name)
                    }
                }
            }
        }
        return byTrack
    }

    @Suppress("DEPRECATION") // DATA is the only way to a file's folder before Android 10.
    private fun read(withAlbumArtist: Boolean): List<Track> {
        val relativePaths = Build.VERSION.SDK_INT >= 29
        val projection = listOfNotNull(
            MediaStore.Audio.Media._ID,
            MediaStore.Audio.Media.TITLE,
            MediaStore.Audio.Media.ARTIST,
            MediaStore.Audio.Media.ALBUM,
            MediaStore.Audio.Media.ALBUM_ID,
            MediaStore.Audio.Media.DURATION,
            MediaStore.Audio.Media.TRACK,
            MediaStore.Audio.Media.YEAR,
            if (relativePaths) MediaStore.Audio.Media.RELATIVE_PATH else MediaStore.Audio.Media.DATA,
            MediaStore.Audio.Media.DISPLAY_NAME,
            MediaStore.Audio.Media.DATE_ADDED,
            ALBUM_ARTIST.takeIf { withAlbumArtist },
            GENRE.takeIf { Build.VERSION.SDK_INT >= 30 },
        ).toTypedArray()
        val tracks = ArrayList<Track>()
        context.contentResolver
            .query(audioCollection(), projection, "${MediaStore.Audio.Media.IS_MUSIC} != 0", null, null)
            ?.use { c ->
                val id = c.getColumnIndexOrThrow(MediaStore.Audio.Media._ID)
                val title = c.getColumnIndexOrThrow(MediaStore.Audio.Media.TITLE)
                val artist = c.getColumnIndexOrThrow(MediaStore.Audio.Media.ARTIST)
                val album = c.getColumnIndexOrThrow(MediaStore.Audio.Media.ALBUM)
                val albumId = c.getColumnIndexOrThrow(MediaStore.Audio.Media.ALBUM_ID)
                val duration = c.getColumnIndexOrThrow(MediaStore.Audio.Media.DURATION)
                val number = c.getColumnIndexOrThrow(MediaStore.Audio.Media.TRACK)
                val year = c.getColumnIndexOrThrow(MediaStore.Audio.Media.YEAR)
                val folder = c.getColumnIndexOrThrow(projection[8])
                val fileName = c.getColumnIndexOrThrow(MediaStore.Audio.Media.DISPLAY_NAME)
                val added = c.getColumnIndexOrThrow(MediaStore.Audio.Media.DATE_ADDED)
                val albumArtist = if (withAlbumArtist) c.getColumnIndexOrThrow(ALBUM_ARTIST) else -1
                val genre = c.getColumnIndex(GENRE)
                while (c.moveToNext()) {
                    // TRACK holds the disc in its thousands: 2005 is disc 2, track 5.
                    val discAndTrack = c.getInt(number)
                    val place = c.getString(folder).orEmpty()
                    tracks += Track(
                        id = c.getLong(id),
                        title = c.getString(title).orEmpty(),
                        artist = c.text(artist),
                        albumArtist = if (albumArtist >= 0) c.text(albumArtist) else null,
                        album = c.text(album),
                        albumId = c.getLong(albumId),
                        durationMs = c.getLong(duration),
                        discNumber = (discAndTrack / 1000).takeIf { it > 0 },
                        trackNumber = (discAndTrack % 1000).takeIf { it > 0 },
                        year = c.getInt(year).takeIf { it > 0 },
                        folder = if (relativePaths) place else File(place).parent.orEmpty(),
                        fileName = c.getString(fileName).orEmpty(),
                        addedAt = c.getLong(added),
                        genres = if (genre >= 0) c.text(genre)?.let { splitGenres(listOf(it)) }.orEmpty() else emptyList(),
                    )
                }
            }
        return tracks
    }

    private companion object {
        /** A public column since Android 11, already there (unlisted) in older MediaStores. */
        const val ALBUM_ARTIST = "album_artist"

        /** A column since Android 11 (MediaStore.Audio.Media.GENRE). */
        const val GENRE = "genre"
    }
}

/** Every audio file MediaStore knows, on all volumes. */
fun audioCollection(): Uri =
    if (Build.VERSION.SDK_INT >= 29) {
        MediaStore.Audio.Media.getContentUri(MediaStore.VOLUME_EXTERNAL)
    } else {
        MediaStore.Audio.Media.EXTERNAL_CONTENT_URI
    }

/** A track's audio file. */
fun trackUri(trackId: Long): Uri = ContentUris.withAppendedId(audioCollection(), trackId)

/** A text column, without MediaStore's "<unknown>" placeholder. */
private fun Cursor.text(column: Int): String? =
    getString(column)?.trim()?.takeIf { it.isNotEmpty() && it != MediaStore.UNKNOWN_STRING }
