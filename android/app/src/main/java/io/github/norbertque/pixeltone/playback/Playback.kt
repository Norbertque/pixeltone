package io.github.norbertque.pixeltone.playback

import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import io.github.norbertque.pixeltone.library.AlbumCover
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.library.albumArtUri
import io.github.norbertque.pixeltone.library.albumIdOf
import io.github.norbertque.pixeltone.library.trackUri
import io.github.norbertque.pixeltone.remote.PcCover
import io.github.norbertque.pixeltone.remote.pcCoverKeyOf

/** A track in the queue, as the player describes it. */
data class QueueEntry(
    /** The phone's MediaStore id; -1 for a track of the PC's own. */
    val trackId: Long,
    val title: String,
    val artist: String?,
    val album: String?,
    /** The cover as an image for Coil ([AlbumCover], [PcCover]), or null. */
    val cover: Any?,
)

data class PlaybackState(
    val queue: List<QueueEntry> = emptyList(),
    /** Where the playing track is in [queue]; -1 when the queue is empty. */
    val index: Int = -1,
    val isPlaying: Boolean = false,
    val durationMs: Long = 0,
    val repeatMode: Int = Player.REPEAT_MODE_OFF,
    val shuffle: Boolean = false,
) {
    val current: QueueEntry? get() = queue.getOrNull(index)
}

/** Repeat goes off → all → one → off, as on Windows. */
fun nextRepeatMode(mode: Int): Int =
    when (mode) {
        Player.REPEAT_MODE_OFF -> Player.REPEAT_MODE_ALL
        Player.REPEAT_MODE_ALL -> Player.REPEAT_MODE_ONE
        else -> Player.REPEAT_MODE_OFF
    }

/** What the player screens may ask of the player. */
interface PlayerControls {
    fun toggle()
    fun next()
    fun previous()
    fun seekTo(positionMs: Long)
    fun jumpTo(index: Int)
    fun move(from: Int, to: Int)
    fun remove(index: Int)
    fun cycleRepeat()
    fun toggleShuffle()
}

/** The media id is the MediaStore track id: the session finds the file from it. */
fun Track.toMediaItem(): MediaItem =
    MediaItem.Builder()
        .setMediaId(id.toString())
        .setUri(trackUri(id))
        .setMediaMetadata(
            MediaMetadata.Builder()
                .setTitle(title)
                .setArtist(artist)
                .setAlbumTitle(album)
                .setAlbumArtist(albumArtist)
                .setTrackNumber(trackNumber)
                .setDiscNumber(discNumber)
                .setReleaseYear(year)
                .setArtworkUri(albumArtUri(albumId))
                .setIsPlayable(true)
                .setIsBrowsable(false)
                .build(),
        )
        .build()

fun MediaItem.toQueueEntry(): QueueEntry =
    QueueEntry(
        trackId = mediaId.toLongOrNull() ?: -1,
        title = mediaMetadata.title?.toString().orEmpty(),
        artist = mediaMetadata.artist?.toString(),
        album = mediaMetadata.albumTitle?.toString(),
        cover = mediaMetadata.artworkUri?.let { uri -> albumIdOf(uri)?.let(::AlbumCover) ?: pcCoverKeyOf(uri)?.let(::PcCover) },
    )
