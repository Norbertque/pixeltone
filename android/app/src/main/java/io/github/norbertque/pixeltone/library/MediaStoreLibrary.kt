package io.github.norbertque.pixeltone.library

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
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/** The phone's music as MediaStore knows it, read again whenever MediaStore changes. */
class MediaStoreLibrary(private val context: Context) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val changes = Channel<Unit>(Channel.CONFLATED)
    private val current = MutableStateFlow<LibraryIndex?>(null)
    private var started = false

    /** Null until the first read is done. */
    val index: StateFlow<LibraryIndex?> = current.asStateFlow()

    /** Starts reading; call once the app may read audio files. */
    @MainThread
    fun start() {
        if (started) return
        started = true
        context.contentResolver.registerContentObserver(
            collection(),
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
                current.value = LibraryIndex(readTracks())
                // MediaStore reports changes in bursts while it scans: read at most once a second.
                delay(1_000)
            }
        }
    }

    private fun readTracks(): List<Track> =
        try {
            read(withAlbumArtist = true)
        } catch (_: IllegalArgumentException) {
            read(withAlbumArtist = false)
        } catch (_: SQLiteException) {
            read(withAlbumArtist = false)
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
            ALBUM_ARTIST.takeIf { withAlbumArtist },
        ).toTypedArray()
        val tracks = ArrayList<Track>()
        context.contentResolver
            .query(collection(), projection, "${MediaStore.Audio.Media.IS_MUSIC} != 0", null, null)
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
                val albumArtist = if (withAlbumArtist) c.getColumnIndexOrThrow(ALBUM_ARTIST) else -1
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
                    )
                }
            }
        return tracks
    }

    private fun collection(): Uri =
        if (Build.VERSION.SDK_INT >= 29) {
            MediaStore.Audio.Media.getContentUri(MediaStore.VOLUME_EXTERNAL)
        } else {
            MediaStore.Audio.Media.EXTERNAL_CONTENT_URI
        }

    private companion object {
        /** A public column since Android 11, already there (unlisted) in older MediaStores. */
        const val ALBUM_ARTIST = "album_artist"
    }
}

/** A text column, without MediaStore's "<unknown>" placeholder. */
private fun Cursor.text(column: Int): String? =
    getString(column)?.trim()?.takeIf { it.isNotEmpty() && it != MediaStore.UNKNOWN_STRING }
