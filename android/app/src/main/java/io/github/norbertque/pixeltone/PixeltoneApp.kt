package io.github.norbertque.pixeltone

import android.app.Application
import coil.ImageLoader
import coil.ImageLoaderFactory
import io.github.norbertque.pixeltone.library.AlbumCoverFetcher
import io.github.norbertque.pixeltone.library.AlbumCoverKeyer
import io.github.norbertque.pixeltone.library.MediaStoreLibrary
import io.github.norbertque.pixeltone.playback.PlayerConnection

class PixeltoneApp : Application(), ImageLoaderFactory {
    val library: MediaStoreLibrary by lazy { MediaStoreLibrary(this) }
    val player: PlayerConnection by lazy { PlayerConnection(this) }

    override fun newImageLoader(): ImageLoader =
        ImageLoader.Builder(this)
            .components {
                add(AlbumCoverKeyer())
                add(AlbumCoverFetcher.Factory(this@PixeltoneApp))
            }
            .crossfade(true)
            .build()
}
