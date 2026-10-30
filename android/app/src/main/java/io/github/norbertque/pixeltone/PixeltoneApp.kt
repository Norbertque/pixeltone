package io.github.norbertque.pixeltone

import android.app.Application
import coil.ImageLoader
import coil.ImageLoaderFactory
import io.github.norbertque.pixeltone.jam.JamControl
import io.github.norbertque.pixeltone.library.AlbumCoverFetcher
import io.github.norbertque.pixeltone.library.AlbumCoverKeyer
import io.github.norbertque.pixeltone.library.History
import io.github.norbertque.pixeltone.library.MediaStoreLibrary
import io.github.norbertque.pixeltone.library.Mixes
import io.github.norbertque.pixeltone.library.mixesOf
import io.github.norbertque.pixeltone.library.UserLists
import io.github.norbertque.pixeltone.lyrics.LyricsFinder
import io.github.norbertque.pixeltone.playback.PlayerConnection
import io.github.norbertque.pixeltone.playback.SleepTimer
import io.github.norbertque.pixeltone.remote.PcCoverFetcher
import io.github.norbertque.pixeltone.remote.PcCoverKeyer
import io.github.norbertque.pixeltone.remote.RemoteControl
import io.github.norbertque.pixeltone.update.Updater
import io.github.norbertque.pixeltone.usb.PreferencesUsbSettings
import io.github.norbertque.pixeltone.usb.UsbDeviceManager
import io.github.norbertque.pixeltone.usb.UsbExclusiveModeController
import io.github.norbertque.pixeltone.usb.UsbSettings
import io.github.norbertque.pixeltone.usbaudio.UsbAudioSession
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.stateIn

class PixeltoneApp : Application(), ImageLoaderFactory {
    val library: MediaStoreLibrary by lazy { MediaStoreLibrary(this) }
    val player: PlayerConnection by lazy { PlayerConnection(this) }
    val remote: RemoteControl by lazy { RemoteControl(this) }
    val jam: JamControl by lazy { JamControl(this, remote) }
    val lists: UserLists by lazy { UserLists(File(filesDir, "lists.json")) }
    val history: History by lazy { History(File(filesDir, "history.jsonl")) }
    val lyrics: LyricsFinder by lazy { LyricsFinder(this) }
    val sleep = SleepTimer()
    /** Today's mix and the genres' mixes; null until the library has been read. */
    val mixes: StateFlow<Mixes?> by lazy {
        mixesOf(library.index, history.listens, lists.lists)
            .stateIn(CoroutineScope(SupervisorJob() + Dispatchers.Default), SharingStarted.WhileSubscribed(5_000), null)
    }
    val updater: Updater by lazy { Updater(this, BuildConfig.UPDATE_FEED, BuildConfig.VERSION_CODE) }
    val usbSettings: UsbSettings by lazy { PreferencesUsbSettings(this) }

    /** Exclusive USB output: follows the DAC and its permission for as long as the app runs. */
    val usb: UsbExclusiveModeController by lazy {
        UsbExclusiveModeController(
            UsbDeviceManager(this),
            usbSettings,
            CoroutineScope(SupervisorJob() + Dispatchers.Default),
            sessionFactory = { connection, capabilities -> UsbAudioSession.open(connection, capabilities) },
        )
    }

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
