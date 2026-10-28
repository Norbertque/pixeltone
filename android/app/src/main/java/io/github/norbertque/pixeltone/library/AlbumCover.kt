package io.github.norbertque.pixeltone.library

import android.content.ContentUris
import android.content.Context
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

class AlbumCoverKeyer : Keyer<AlbumCover> {
    override fun key(data: AlbumCover, options: Options): String = "album-cover:${data.albumId}"
}

/** Loads album covers from MediaStore's thumbnails (Android 10+) or its album art provider. */
class AlbumCoverFetcher(
    private val context: Context,
    private val cover: AlbumCover,
    private val options: Options,
) : Fetcher {
    override suspend fun fetch(): FetchResult = withContext(Dispatchers.IO) {
        val px = (options.size.width as? Dimension.Pixels)?.px ?: DEFAULT_PX
        val bitmap = if (Build.VERSION.SDK_INT >= 29) {
            val albums = MediaStore.Audio.Albums.getContentUri(MediaStore.VOLUME_EXTERNAL)
            context.contentResolver.loadThumbnail(ContentUris.withAppendedId(albums, cover.albumId), Size(px, px), null)
        } else {
            val uri = ContentUris.withAppendedId(LEGACY_ALBUM_ART, cover.albumId)
            context.contentResolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it) }
                ?: throw FileNotFoundException(uri.toString())
        }
        DrawableResult(bitmap.toDrawable(context.resources), isSampled = true, dataSource = DataSource.DISK)
    }

    class Factory(private val context: Context) : Fetcher.Factory<AlbumCover> {
        override fun create(data: AlbumCover, options: Options, imageLoader: ImageLoader): Fetcher =
            AlbumCoverFetcher(context, data, options)
    }

    private companion object {
        const val DEFAULT_PX = 512
        val LEGACY_ALBUM_ART: Uri = "content://media/external/audio/albumart".toUri()
    }
}
