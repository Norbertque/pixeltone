package io.github.norbertque.pixeltone.playback

import java.io.File
import org.json.JSONArray
import org.json.JSONObject

/** The queue when playback last changed, so the next start picks it up. */
data class SavedQueue(
    val trackIds: List<Long>,
    val index: Int,
    val positionMs: Long,
    val repeatMode: Int,
    val shuffle: Boolean,
)

class QueueStore(private val file: File) {
    fun load(): SavedQueue? =
        try {
            val json = JSONObject(file.readText())
            val ids = json.getJSONArray("trackIds")
            SavedQueue(
                trackIds = List(ids.length()) { ids.getLong(it) },
                index = json.optInt("index"),
                positionMs = json.optLong("positionMs"),
                repeatMode = json.optInt("repeatMode"),
                shuffle = json.optBoolean("shuffle"),
            )
        } catch (_: Exception) {
            null
        }

    fun save(queue: SavedQueue) {
        val json = JSONObject()
            .put("trackIds", JSONArray(queue.trackIds))
            .put("index", queue.index)
            .put("positionMs", queue.positionMs)
            .put("repeatMode", queue.repeatMode)
            .put("shuffle", queue.shuffle)
        // Written beside the old file and swapped in, so a crash never leaves half a queue.
        val partial = File(file.parentFile, file.name + ".part")
        partial.writeText(json.toString())
        partial.renameTo(file)
    }
}
