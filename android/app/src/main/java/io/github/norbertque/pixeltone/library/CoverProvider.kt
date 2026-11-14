package io.github.norbertque.pixeltone.library

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.graphics.Bitmap
import android.net.Uri
import android.os.ParcelFileDescriptor
import java.io.File
import java.io.FileNotFoundException
import java.io.IOException
import java.util.concurrent.atomic.AtomicLong

/**
 * Album covers at [albumArtUri]s, for the cars the media session lets see them (Android Auto
 * can't read MediaStore's thumbnails itself): a JPEG of the album's thumbnail, made again after
 * a day. Not exported: only a grant of the app's opens one.
 */
class CoverProvider : ContentProvider() {
    override fun onCreate(): Boolean = true

    override fun getType(uri: Uri): String? = if (albumIdOf(uri) != null) "image/jpeg" else null

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        if (mode != "r") throw SecurityException("Covers can only be read")
        val albumId = albumIdOf(uri) ?: throw FileNotFoundException(uri.toString())
        val context = context ?: throw FileNotFoundException(uri.toString())
        val folder = File(context.cacheDir, "covers")
        val file = File(folder, "$albumId.jpg")
        if (!file.isFile || System.currentTimeMillis() - file.lastModified() > DAY_MS) {
            val cover = try {
                loadAlbumArt(context, albumId, SIZE_PX)
            } catch (e: IOException) {
                throw FileNotFoundException("No cover for album $albumId").apply { initCause(e) }
            }
            folder.mkdirs()
            // Written aside and moved in whole, so a reader at the same time never gets half of it.
            val part = File.createTempFile("$albumId-", ".part", folder)
            try {
                part.outputStream().use { cover.compress(Bitmap.CompressFormat.JPEG, QUALITY, it) }
                if (!part.renameTo(file)) throw FileNotFoundException("Couldn't keep the cover of album $albumId")
            } finally {
                part.delete()
            }
            forgetOld(folder)
        }
        return ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
    }

    /**
     * Once a day, drops the covers made over a day ago: one asked for again is made anew anyway,
     * and a car that pages through every album would otherwise leave them all here for good.
     */
    private fun forgetOld(folder: File) {
        val now = System.currentTimeMillis()
        val last = tidied.get()
        if (now - last < DAY_MS || !tidied.compareAndSet(last, now)) return
        folder.listFiles()?.forEach { if (now - it.lastModified() > DAY_MS) it.delete() }
    }

    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, selectionArgs: Array<out String>?, sortOrder: String?): Cursor? = null

    override fun insert(uri: Uri, values: ContentValues?): Uri? = null

    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?): Int = 0

    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?): Int = 0

    private companion object {
        const val SIZE_PX = 512
        const val QUALITY = 90

        /** When the old covers were last dropped. */
        val tidied = AtomicLong()
    }
}
