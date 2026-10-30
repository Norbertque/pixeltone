package io.github.norbertque.pixeltone.playback

import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.annotation.OptIn
import androidx.media3.common.C
import androidx.media3.common.Format
import androidx.media3.common.MimeTypes
import androidx.media3.common.Player
import androidx.media3.common.Tracks
import androidx.media3.common.util.UnstableApi
import io.github.norbertque.pixeltone.library.AlbumCover
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.library.albumArtUri
import io.github.norbertque.pixeltone.library.albumIdOf
import io.github.norbertque.pixeltone.library.trackUri
import io.github.norbertque.pixeltone.remote.PcCover
import io.github.norbertque.pixeltone.remote.pcCoverKeyOf
import io.github.norbertque.pixeltone.usb.formatPcm
import io.github.norbertque.pixeltone.usb.formatRate

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

/** What the playing file holds: its codec, and its bit depth (lossless), sample rate and bitrate when known. */
data class TrackFormat(val codec: String, val bits: Int? = null, val sampleRateHz: Int? = null, val bitrate: Int? = null) {
    /** "FLAC • 24-bit/96 kHz", "MP3 • 320 kbps • 44.1 kHz". */
    val label: String
        get() = listOfNotNull(
            codec,
            if (bits != null && sampleRateHz != null) formatPcm(bits, sampleRateHz) else null,
            bitrate?.takeIf { bits == null }?.let { "${(it + 500) / 1000} kbps" },
            sampleRateHz?.takeIf { bits == null }?.let(::formatRate),
        ).joinToString(" • ")
}

/** The format of the audio [tracks] play now; null when there's none (or it's a PC's). */
@OptIn(UnstableApi::class) // Format's PCM encodings
fun trackFormatOf(tracks: Tracks): TrackFormat? {
    val group = tracks.groups.firstOrNull { it.type == C.TRACK_TYPE_AUDIO && it.isSelected } ?: return null
    val format = (0 until group.length).firstOrNull(group::isTrackSelected)?.let(group::getTrackFormat) ?: return null
    val mime = format.sampleMimeType ?: return null
    return TrackFormat(
        codec = CODECS[mime] ?: mime.substringAfter('/').uppercase(),
        bits = when (format.pcmEncoding) {
            C.ENCODING_PCM_8BIT -> 8
            C.ENCODING_PCM_16BIT, C.ENCODING_PCM_16BIT_BIG_ENDIAN -> 16
            C.ENCODING_PCM_24BIT, C.ENCODING_PCM_24BIT_BIG_ENDIAN -> 24
            C.ENCODING_PCM_32BIT, C.ENCODING_PCM_32BIT_BIG_ENDIAN, C.ENCODING_PCM_FLOAT -> 32
            else -> null
        },
        sampleRateHz = format.sampleRate.takeIf { it != Format.NO_VALUE },
        bitrate = format.bitrate.takeIf { it != Format.NO_VALUE },
    )
}

private val CODECS = mapOf(
    MimeTypes.AUDIO_FLAC to "FLAC",
    MimeTypes.AUDIO_ALAC to "ALAC",
    MimeTypes.AUDIO_RAW to "PCM",
    MimeTypes.AUDIO_MPEG to "MP3",
    MimeTypes.AUDIO_MPEG_L2 to "MP2",
    MimeTypes.AUDIO_AAC to "AAC",
    MimeTypes.AUDIO_OPUS to "Opus",
    MimeTypes.AUDIO_VORBIS to "Vorbis",
    MimeTypes.AUDIO_AC3 to "AC-3",
    MimeTypes.AUDIO_E_AC3 to "E-AC-3",
    MimeTypes.AUDIO_DTS to "DTS",
    MimeTypes.AUDIO_AMR_NB to "AMR",
    MimeTypes.AUDIO_AMR_WB to "AMR-WB",
)

data class PlaybackState(
    val queue: List<QueueEntry> = emptyList(),
    /** Where the playing track is in [queue]; -1 when the queue is empty. */
    val index: Int = -1,
    val isPlaying: Boolean = false,
    val durationMs: Long = 0,
    val repeatMode: Int = Player.REPEAT_MODE_OFF,
    val shuffle: Boolean = false,
    /** The playing file's format, once the player knows it (not a PC's). */
    val format: TrackFormat? = null,
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

    /** Everything but the track that plays leaves the queue. */
    fun clearQueue() {}
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
