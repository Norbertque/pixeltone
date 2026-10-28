package io.github.norbertque.pixeltone.library

import android.content.ContentUris
import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import android.os.Build
import android.provider.MediaStore
import android.util.Size
import androidx.core.graphics.drawable.toDrawable
import androidx.core.net.toUri
import coil.ImageLoader
import coil.decode.DataSource
import coil.fetch.DrawableResult
import coil.fetch.FetchResult
import coil.fetch.Fetcher
import coil.key.Keyer
import coil.request.Options
import coil.size.Dimension
import java.io.FileNotFoundException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** The cover of a MediaStore album, as an image to load with Coil. */
data class AlbumCover(val albumId: Long)

/** MediaStore's album art address; the media session hands covers around by it. */
fun albumArtUri(albumId: Long): Uri = ContentUris.withAppendedId(ALBUM_ART, albumId)

/** The album in an [albumArtUri], or null for any other address. */
fun albumIdOf(uri: Uri): Long? =
    if (uri.authority == ALBUM_ART.authority && uri.path?.startsWith(ALBUM_ART.path + "/") == true) {
        uri.lastPathSegment?.toLongOrNull()
    } else {
        null
    }

/** An album's cover at most [px] wide: MediaStore's thumbnail (Android 10+) or its album art. */
fun loadAlbumArt(context: Context, albumId: Long, px: Int): Bitmap =
    if (Build.VERSION.SDK_INT >= 29) {
        val albums = MediaStore.Audio.Albums.getContentUri(MediaStore.VOLUME_EXTERNAL)
        context.contentResolver.loadThumbnail(ContentUris.withAppendedId(albums, albumId), Size(px, px), null)
    } else {
        val uri = albumArtUri(albumId)
        context.contentResolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it) }
            ?: throw FileNotFoundException(uri.toString())
    }

private val ALBUM_ART: Uri = "content://media/external/audio/albumart".toUri()

class AlbumCoverKeyer : Keyer<AlbumCover> {
    override fun key(data: AlbumCover, options: Options): String = "album-cover:${data.albumId}"
}

class AlbumCoverFetcher(
    private val context: Context,
    private val cover: AlbumCover,
    private val options: Options,
) : Fetcher {
    override suspend fun fetch(): FetchResult = withContext(Dispatchers.IO) {
        val px = (options.size.width as? Dimension.Pixels)?.px ?: DEFAULT_PX
        val bitmap = loadAlbumArt(context, cover.albumId, px)
        DrawableResult(bitmap.toDrawable(context.resources), isSampled = true, dataSource = DataSource.DISK)
    }

    class Factory(private val context: Context) : Fetcher.Factory<AlbumCover> {
        override fun create(data: AlbumCover, options: Options, imageLoader: ImageLoader): Fetcher =
            AlbumCoverFetcher(context, data, options)
    }

    private companion object {
        const val DEFAULT_PX = 512
    }
}
