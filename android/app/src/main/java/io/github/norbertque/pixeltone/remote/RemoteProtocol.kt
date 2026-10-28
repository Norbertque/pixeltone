package io.github.norbertque.pixeltone.remote

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.encodeToJsonElement
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put

/**
 * Pixeltone's remote control protocol, version 1 (`docs/remote-protocol.md`): HTTPS and a
 * WebSocket on the local network. Pixeltone on a PC runs the server; this app finds it over
 * mDNS, pairs with a six-digit code, follows and drives the PC's player and hands its own
 * queue over (the PC fetches the files from this phone and plays them, bit-perfect if it can).
 */
internal val RemoteJson = Json {
    ignoreUnknownKeys = true
    explicitNulls = false
    encodeDefaults = true
}

object RemoteProtocol {
    const val VERSION = 1
    const val SERVICE_TYPE = "_pixeltone._tcp"
    const val DEFAULT_PORT = 47821

    /** Most tracks the PC takes in one hand-over. */
    const val MAX_HAND_OVER = 10_000
}

/** What a PC says about itself in `/v1/info` and `hello`. */
@Serializable
data class PcInfo(
    val app: String = "",
    val id: String,
    val name: String,
    val version: String = "",
    val protocol: Int = 0,
    /** SHA-256 of the PC's certificate (DER), lowercase hex: pinned when pairing. */
    val fingerprint: String,
)

@Serializable
internal data class PairRequest(
    val code: String,
    val deviceId: String,
    val deviceName: String,
)

@Serializable
internal data class PairResponse(
    val token: String,
    val server: PcInfo,
)

/** The PC's volume. `kind`: app (its own level), device (the DAC's), software, fixed. */
@Serializable
data class PcVolume(
    val kind: String,
    val db: Float,
    val min: Float,
    val max: Float,
    val step: Float = 0.5f,
    val muted: Boolean = false,
) {
    val adjustable: Boolean get() = kind != "fixed" && max > min
}

@Serializable
data class PcTrack(
    /** For a phone track, the URL it was handed over with. */
    val path: String = "",
    val title: String = "",
    val artist: String? = null,
    val album: String? = null,
    val durationMs: Double? = null,
    val coverKey: String? = null,
)

@Serializable
data class PcBadge(
    val label: String = "",
    val bitPerfect: Boolean = false,
)

@Serializable
data class PcOutput(
    val deviceName: String = "",
    val badge: PcBadge? = null,
)

@Serializable
data class PcState(
    /** playing, paused or stopped. */
    val status: String = "stopped",
    val track: PcTrack? = null,
    val currentId: Long? = null,
    val queueIndex: Int? = null,
    val queueLength: Int = 0,
    val queueVersion: Long = 0,
    val positionMs: Double = 0.0,
    /** off, all or one. */
    val repeat: String = "off",
    val shuffle: Boolean = false,
    val output: PcOutput? = null,
) {
    val isPlaying: Boolean get() = status == "playing"
}

/** A row of the PC's queue (`/v1/queue`). */
@Serializable
data class PcQueueRow(
    val id: Long,
    val path: String = "",
    val title: String = "",
    val artist: String? = null,
    val album: String? = null,
    val durationMs: Double? = null,
    val coverKey: String? = null,
)

/** A phone track in a hand-over: `url` serves the original file from this phone. */
@Serializable
data class HandOverItem(
    val url: String,
    val artUrl: String? = null,
    val title: String? = null,
    val artist: String? = null,
    val album: String? = null,
    val durationMs: Long? = null,
)

/** Messages from the PC over the WebSocket. */
sealed interface PcMessage {
    data class Hello(val server: PcInfo) : PcMessage
    data class State(val state: PcState) : PcMessage
    data class Position(val positionMs: Double) : PcMessage
    data class Volume(val volume: PcVolume?) : PcMessage
    data object LibraryChanged : PcMessage
    data class Error(val code: String, val message: String) : PcMessage

    companion object {
        /** Null for anything this version doesn't know (newer PCs may send more). */
        fun parse(text: String): PcMessage? = runCatching {
            val obj = RemoteJson.parseToJsonElement(text).jsonObject
            when (obj["type"]?.jsonPrimitive?.content) {
                "hello" -> Hello(RemoteJson.decodeFromJsonElement(PcInfo.serializer(), obj.getValue("server")))
                "state" -> State(RemoteJson.decodeFromJsonElement(PcState.serializer(), obj.getValue("state")))
                "position" -> Position(obj.getValue("positionMs").jsonPrimitive.content.toDouble())
                "volume" -> Volume(
                    obj["volume"]
                        ?.takeIf { it is JsonObject }
                        ?.let { RemoteJson.decodeFromJsonElement(PcVolume.serializer(), it) },
                )
                "libraryChanged" -> LibraryChanged
                "error" -> Error(
                    code = obj["code"]?.jsonPrimitive?.content.orEmpty(),
                    message = obj["message"]?.jsonPrimitive?.content.orEmpty(),
                )
                else -> null
            }
        }.getOrNull()
    }
}

/** Commands to the PC over the WebSocket. */
object PcCommand {
    fun simple(cmd: String): String = buildJsonObject { put("cmd", cmd) }.toString()

    fun seek(ms: Long): String = buildJsonObject {
        put("cmd", "seek")
        put("ms", ms)
    }.toString()

    fun setRepeat(mode: String): String = buildJsonObject {
        put("cmd", "setRepeat")
        put("mode", mode)
    }.toString()

    fun setShuffle(on: Boolean): String = buildJsonObject {
        put("cmd", "setShuffle")
        put("on", on)
    }.toString()

    fun jump(id: Long): String = buildJsonObject {
        put("cmd", "jump")
        put("id", id)
    }.toString()

    fun setVolume(db: Float): String = buildJsonObject {
        put("cmd", "setVolume")
        put("db", db)
    }.toString()

    fun setMuted(muted: Boolean): String = buildJsonObject {
        put("cmd", "setMuted")
        put("muted", muted)
    }.toString()

    /** A hand-over: the PC replaces its queue with [items] and plays from [start]. */
    fun load(
        items: List<HandOverItem>,
        start: Int,
        positionMs: Long,
        play: Boolean,
        repeat: String,
    ): String = JsonObject(
        mapOf(
            "cmd" to JsonPrimitive("load"),
            "items" to JsonArray(items.map { RemoteJson.encodeToJsonElement(it) }),
            "start" to JsonPrimitive(start),
            "positionMs" to JsonPrimitive(positionMs),
            "play" to JsonPrimitive(play),
            "repeat" to JsonPrimitive(repeat),
        ),
    ).toString()
}

/** An address typed by hand, as Pixeltone shows it (`ip:port`, the port optional). */
fun parseAddress(input: String): Pair<String, Int>? {
    val text = input.trim().removePrefix("https://").removePrefix("http://").trimEnd('/')
    if (text.isEmpty() || text.any(Char::isWhitespace)) return null
    val (host, port) = when {
        text.startsWith("[") -> {
            val end = text.indexOf(']').takeIf { it > 1 } ?: return null
            val rest = text.substring(end + 1)
            text.substring(1, end) to if (rest.isEmpty()) null else rest.removePrefix(":").toIntOrNull() ?: return null
        }
        text.count { it == ':' } == 1 -> {
            val (h, p) = text.split(':')
            h to (p.toIntOrNull() ?: return null)
        }
        else -> text to null
    }
    if (host.isEmpty()) return null
    val resolvedPort = port ?: RemoteProtocol.DEFAULT_PORT
    if (resolvedPort !in 1..65_535) return null
    return host to resolvedPort
}
