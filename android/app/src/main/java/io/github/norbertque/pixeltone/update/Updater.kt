package io.github.norbertque.pixeltone.update

import android.app.PendingIntent
import android.content.Context
import androidx.core.content.edit
import android.content.Intent
import android.content.pm.PackageInstaller
import android.os.Build
import java.io.File
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONException
import org.json.JSONObject

/** What the update feed says about the newest build. */
data class UpdateInfo(val versionCode: Int, val versionName: String, val url: String, val sha256: String)

sealed interface UpdateState {
    data object Idle : UpdateState
    data class Available(val info: UpdateInfo) : UpdateState
    data class Downloading(val info: UpdateInfo, val progress: Float) : UpdateState
    data object Installing : UpdateState
    data class Failed(val message: String) : UpdateState
}

/** The feed's JSON, or null when it isn't a complete one. */
fun parseUpdateInfo(json: String): UpdateInfo? =
    try {
        val feed = JSONObject(json)
        UpdateInfo(
            versionCode = feed.getInt("versionCode"),
            versionName = feed.getString("versionName"),
            url = feed.getString("url"),
            sha256 = feed.getString("sha256").lowercase(),
        ).takeIf { it.url.startsWith("https://") && it.sha256.matches(SHA256) }
    } catch (_: JSONException) {
        null
    }

private val SHA256 = Regex("[0-9a-f]{64}")

/** A file's SHA-256 in lowercase hex. */
fun sha256(file: File): String {
    val digest = MessageDigest.getInstance("SHA-256")
    file.inputStream().use { input ->
        val buffer = ByteArray(64 * 1024)
        while (true) {
            val read = input.read(buffer)
            if (read < 0) break
            digest.update(buffer, 0, read)
        }
    }
    return digest.digest().toHex()
}

private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }

/**
 * Finds newer builds in the update feed (the `android-dev` release on GitHub), downloads one,
 * checks it against the feed's SHA-256 and hands it to Android's installer, which asks the
 * user to confirm. Debug builds have no feed and never update.
 */
class Updater(private val context: Context, private val feedUrl: String, private val versionCode: Int) {
    private val scope = MainScope()
    private val prefs = context.getSharedPreferences("updates", Context.MODE_PRIVATE)
    private val current = MutableStateFlow<UpdateState>(UpdateState.Idle)
    private val looking = MutableStateFlow(prefs.getBoolean(AUTOMATIC, true))
    private var checked = false

    val state: StateFlow<UpdateState> = current.asStateFlow()

    /** Whether Pixeltone looks for a newer build each time it starts. */
    val automatic: StateFlow<Boolean> = looking.asStateFlow()

    fun setAutomatic(on: Boolean) {
        prefs.edit { putBoolean(AUTOMATIC, on) }
        looking.value = on
    }

    /** Looks for a newer build once per start of the app; quietly gives up when offline. */
    fun checkOnce() {
        if (checked || feedUrl.isEmpty() || !looking.value) return
        checked = true
        scope.launch {
            val info = runCatching { withContext(Dispatchers.IO) { fetch() } }.getOrNull() ?: return@launch
            if (info.versionCode > versionCode) current.value = UpdateState.Available(info)
        }
    }

    /** Looks now: true when a newer build is offered, false when this is the newest, null when the feed can't be read. */
    suspend fun checkNow(): Boolean? {
        if (feedUrl.isEmpty()) return null
        val info = runCatching { withContext(Dispatchers.IO) { fetch() } }.getOrNull() ?: return null
        if (info.versionCode <= versionCode) return false
        current.value = UpdateState.Available(info)
        return true
    }

    fun dismiss() {
        current.value = UpdateState.Idle
    }

    fun update(info: UpdateInfo) {
        scope.launch {
            try {
                current.value = UpdateState.Downloading(info, 0f)
                val apk = withContext(Dispatchers.IO) {
                    download(info) { current.value = UpdateState.Downloading(info, it) }
                }
                current.value = UpdateState.Installing
                withContext(Dispatchers.IO) { install(apk) }
            } catch (e: Exception) {
                current.value = UpdateState.Failed(e.message ?: e.javaClass.simpleName)
            }
        }
    }

    /** Android's installer reports here through [InstallReceiver]; null means done or cancelled. */
    fun installFinished(error: String?) {
        current.value = if (error == null) UpdateState.Idle else UpdateState.Failed(error)
    }

    private fun fetch(): UpdateInfo? {
        val connection = URL(feedUrl).openConnection() as HttpURLConnection
        connection.connectTimeout = 10_000
        connection.readTimeout = 10_000
        return try {
            if (connection.responseCode == HttpURLConnection.HTTP_OK) {
                parseUpdateInfo(connection.inputStream.bufferedReader().readText())
            } else {
                null
            }
        } finally {
            connection.disconnect()
        }
    }

    private fun download(info: UpdateInfo, onProgress: (Float) -> Unit): File {
        val dir = File(context.cacheDir, "updates")
        dir.mkdirs()
        dir.listFiles()?.forEach { it.delete() }
        val apk = File(dir, "pixeltone-${info.versionCode}.apk")
        val digest = MessageDigest.getInstance("SHA-256")
        val connection = URL(info.url).openConnection() as HttpURLConnection
        connection.connectTimeout = 15_000
        connection.readTimeout = 30_000
        try {
            if (connection.responseCode != HttpURLConnection.HTTP_OK) throw IOException("HTTP ${connection.responseCode}")
            val total = connection.contentLengthLong
            var done = 0L
            var shown = -1
            connection.inputStream.use { input ->
                apk.outputStream().use { output ->
                    val buffer = ByteArray(64 * 1024)
                    while (true) {
                        val read = input.read(buffer)
                        if (read < 0) break
                        output.write(buffer, 0, read)
                        digest.update(buffer, 0, read)
                        done += read
                        val percent = if (total > 0) (done * 100 / total).toInt() else -1
                        if (percent != shown) {
                            shown = percent
                            onProgress(percent.coerceAtLeast(0) / 100f)
                        }
                    }
                }
            }
        } finally {
            connection.disconnect()
        }
        if (digest.digest().toHex() != info.sha256) {
            apk.delete()
            throw IOException("The download doesn't match the update feed; try again later.")
        }
        return apk
    }

    private fun install(apk: File) {
        val installer = context.packageManager.packageInstaller
        val sessionId = installer.createSession(PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL))
        installer.openSession(sessionId).use { session ->
            apk.inputStream().use { input ->
                session.openWrite("pixeltone.apk", 0, apk.length()).use { output ->
                    input.copyTo(output)
                    session.fsync(output)
                }
            }
            // Mutable: the installer adds the outcome to it.
            val flags = PendingIntent.FLAG_UPDATE_CURRENT or (if (Build.VERSION.SDK_INT >= 31) PendingIntent.FLAG_MUTABLE else 0)
            val result = PendingIntent.getBroadcast(context, sessionId, Intent(context, InstallReceiver::class.java), flags)
            session.commit(result.intentSender)
        }
    }
}

private const val AUTOMATIC = "automatic"
