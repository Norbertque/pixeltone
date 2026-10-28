package io.github.norbertque.pixeltone.playback

import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.library.albumArtUri
import io.github.norbertque.pixeltone.library.albumIdOf
import io.github.norbertque.pixeltone.library.trackUri

/** A track in the queue, as the player describes it. */
data class QueueEntry(
    val trackId: Long,
    val title: String,
    val artist: String?,
    val album: String?,
    val albumId: Long,
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
        albumId = mediaMetadata.artworkUri?.let(::albumIdOf) ?: -1,
    )
