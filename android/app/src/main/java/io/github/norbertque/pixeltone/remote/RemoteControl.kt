package io.github.norbertque.pixeltone.remote

import android.content.Context
import android.graphics.Bitmap
import android.net.Uri
import io.github.norbertque.pixeltone.library.loadAlbumArt
import io.github.norbertque.pixeltone.library.trackUri
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileNotFoundException

/** The playback service, while it runs: it moves playback between the phone and a PC. */
interface RemoteHost {
    fun playOn(pc: PairedPc)

    fun playHere()
}

/**
 * Everything about Pixeltone on PCs in one place: paired PCs, finding them, the connection and
 * the phone's file server. Playback moves through the [host] (the playback service).
 */
class RemoteControl(context: Context) {
    val paired = PairedPcs(File(context.noBackupFilesDir, "paired-pcs.json"))
    val discovery = PcDiscovery(context)
    val connection = PcConnection(context, paired)
    val files = PhoneFileServer(MediaStoreFiles(context))

    @Volatile
    var host: RemoteHost? = null

    /** Moves playback to [pc], at the address it was [found] at if it moved. */
    fun playOn(pc: PairedPc, found: FoundPc? = null) {
        found?.let { paired.moved(pc.id, it.host, it.port) }
        val current = paired.find(pc.id) ?: pc
        host?.playOn(current) ?: connection.begin(current)
    }

    /** Brings playback back to the phone. */
    fun playHere() {
        host?.playHere() ?: connection.end()
    }

    suspend fun lookUp(host: String, port: Int): PcInfo = connection.info(host, port)

    suspend fun pair(host: String, port: Int, code: String): PairedPc = connection.pair(host, port, code)

    fun forget(pc: PairedPc) = connection.forget(pc)
}

/** The PC's cover named [key], as an image to load with Coil. */
data class PcCover(val key: String)

/** A PC's cover as an address the media session can hand around (see `AlbumArtBitmapLoader`). */
fun pcCoverUri(key: String): Uri =
    Uri.Builder().scheme(PC_COVER_SCHEME).authority("cover").appendQueryParameter("key", key).build()

/** The key in a [pcCoverUri], or null for any other address. */
fun pcCoverKeyOf(uri: Uri): String? =
    if (uri.scheme == PC_COVER_SCHEME && uri.authority == "cover") uri.getQueryParameter("key") else null

private const val PC_COVER_SCHEME = "pixeltone-pc"

/** Phone tracks for the PC, straight from MediaStore. */
private class MediaStoreFiles(private val context: Context) : PhoneFiles {
    override fun track(trackId: Long): ServedFile? {
        val uri = trackUri(trackId)
        val resolver = context.contentResolver
        val size = runCatching {
            resolver.openAssetFileDescriptor(uri, "r")?.use { it.length.takeIf { l -> l >= 0 } ?: it.parcelFileDescriptor.statSize }
        }.getOrNull()?.takeIf { it >= 0 } ?: return null
        val mimeType = resolver.getType(uri) ?: "application/octet-stream"
        return ServedFile(size, mimeType) { resolver.openInputStream(uri) ?: throw FileNotFoundException(uri.toString()) }
    }

    override fun cover(albumId: Long): ByteArray? = runCatching {
        val bitmap = loadAlbumArt(context, albumId, COVER_PX)
        ByteArrayOutputStream().also { bitmap.compress(Bitmap.CompressFormat.JPEG, 90, it) }.toByteArray()
    }.getOrNull()

    private companion object {
        const val COVER_PX = 1000
    }
}
