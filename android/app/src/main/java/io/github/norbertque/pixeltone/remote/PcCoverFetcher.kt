package io.github.norbertque.pixeltone.remote

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import androidx.core.graphics.drawable.toDrawable
import coil.ImageLoader
import coil.decode.DataSource
import coil.fetch.DrawableResult
import coil.fetch.FetchResult
import coil.fetch.Fetcher
import coil.key.Keyer
import coil.request.Options
import java.io.IOException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** A cover of the PC that playback is on, fetched over the pinned connection. */
fun RemoteControl.loadPcCover(key: String): Bitmap {
    val pc = connection.session.value?.pc ?: throw IOException("Playback isn't on a PC.")
    val bytes = connection.cover(pc, key)
    return BitmapFactory.decodeByteArray(bytes, 0, bytes.size) ?: throw IOException("The PC's cover isn't an image.")
}

class PcCoverKeyer(private val remote: RemoteControl) : Keyer<PcCover> {
    // The same key means another picture on another PC.
    override fun key(data: PcCover, options: Options): String = "pc-cover:${remote.connection.session.value?.pc?.id}:${data.key}"
}

class PcCoverFetcher(private val context: Context, private val remote: RemoteControl, private val cover: PcCover) : Fetcher {
    override suspend fun fetch(): FetchResult = withContext(Dispatchers.IO) {
        DrawableResult(remote.loadPcCover(cover.key).toDrawable(context.resources), isSampled = false, dataSource = DataSource.NETWORK)
    }

    class Factory(private val context: Context, private val remote: RemoteControl) : Fetcher.Factory<PcCover> {
        override fun create(data: PcCover, options: Options, imageLoader: ImageLoader): Fetcher = PcCoverFetcher(context, remote, data)
    }
}
