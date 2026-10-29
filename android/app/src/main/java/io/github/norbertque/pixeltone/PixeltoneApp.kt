package io.github.norbertque.pixeltone

import android.app.Application
import coil.ImageLoader
import coil.ImageLoaderFactory
import io.github.norbertque.pixeltone.jam.JamControl
import io.github.norbertque.pixeltone.library.AlbumCoverFetcher
import io.github.norbertque.pixeltone.library.AlbumCoverKeyer
import io.github.norbertque.pixeltone.library.MediaStoreLibrary
import io.github.norbertque.pixeltone.playback.PlayerConnection
import io.github.norbertque.pixeltone.remote.PcCoverFetcher
import io.github.norbertque.pixeltone.remote.PcCoverKeyer
import io.github.norbertque.pixeltone.remote.RemoteControl
import io.github.norbertque.pixeltone.update.Updater

class PixeltoneApp : Application(), ImageLoaderFactory {
    val library: MediaStoreLibrary by lazy { MediaStoreLibrary(this) }
    val player: PlayerConnection by lazy { PlayerConnection(this) }
    val remote: RemoteControl by lazy { RemoteControl(this) }
    val jam: JamControl by lazy { JamControl(this, remote) }
    val updater: Updater by lazy { Updater(this, BuildConfig.UPDATE_FEED, BuildConfig.VERSION_CODE) }

    override fun newImageLoader(): ImageLoader =
        ImageLoader.Builder(this)
            .components {
                add(AlbumCoverKeyer())
                add(AlbumCoverFetcher.Factory(this@PixeltoneApp))
                add(PcCoverKeyer(remote))
                add(PcCoverFetcher.Factory(this@PixeltoneApp, remote))
            }
            .crossfade(true)
            .build()
}
