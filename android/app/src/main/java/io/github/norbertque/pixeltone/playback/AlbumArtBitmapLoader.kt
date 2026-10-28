package io.github.norbertque.pixeltone.playback

import android.content.Context
import android.graphics.Bitmap
import android.net.Uri
import androidx.annotation.OptIn
import androidx.media3.common.util.BitmapLoader
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.DataSourceBitmapLoader
import com.google.common.util.concurrent.ListenableFuture
import com.google.common.util.concurrent.MoreExecutors
import io.github.norbertque.pixeltone.library.albumIdOf
import io.github.norbertque.pixeltone.library.loadAlbumArt
import io.github.norbertque.pixeltone.remote.RemoteControl
import io.github.norbertque.pixeltone.remote.loadPcCover
import io.github.norbertque.pixeltone.remote.pcCoverKeyOf
import java.util.concurrent.Callable
import java.util.concurrent.Executors

/**
 * Covers for the media notification and lock screen: album art addresses go through
 * MediaStore's thumbnails (the old album art provider is gone on newer Android), a PC's covers
 * over the connection to it, everything else (pictures embedded in the files) through Media3's
 * own loader.
 */
@OptIn(UnstableApi::class)
class AlbumArtBitmapLoader(private val context: Context, private val remote: RemoteControl) : BitmapLoader {
    private val executor = MoreExecutors.listeningDecorator(Executors.newSingleThreadExecutor())
    private val others = DataSourceBitmapLoader.Builder(context).build()

    override fun supportsMimeType(mimeType: String): Boolean = others.supportsMimeType(mimeType)

    override fun decodeBitmap(data: ByteArray): ListenableFuture<Bitmap> = others.decodeBitmap(data)

    override fun loadBitmap(uri: Uri): ListenableFuture<Bitmap> {
        albumIdOf(uri)?.let { albumId -> return executor.submit(Callable { loadAlbumArt(context, albumId, SIZE_PX) }) }
        pcCoverKeyOf(uri)?.let { key -> return executor.submit(Callable { remote.loadPcCover(key) }) }
        return others.loadBitmap(uri)
    }

    private companion object {
        const val SIZE_PX = 720
    }
}
