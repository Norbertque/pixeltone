package io.github.norbertque.pixeltone.playback

import android.content.Context
import androidx.core.content.edit
import kotlin.math.abs
import kotlin.math.round
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

/** The equaliser's band centres in Hz, as on Windows (`pp-core` eq.rs `BANDS`). */
val EQ_BANDS = listOf(31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000)

/** A band's gain limit, dB. */
const val EQ_MAX_DB = 12f

/** The built-in curves, as on Windows (`ui/src/lib/eq.ts`). */
val EQ_PRESETS: List<Pair<String, List<Float>>> = listOf(
    "Flat" to listOf(0f, 0f, 0f, 0f, 0f, 0f, 0f, 0f, 0f, 0f),
    "Bass boost" to listOf(6f, 5f, 4f, 2f, 0.5f, 0f, 0f, 0f, 0f, 0f),
    "Treble boost" to listOf(0f, 0f, 0f, 0f, 0f, 0.5f, 2f, 3.5f, 5f, 6f),
    "Vocal" to listOf(-2f, -2f, -1f, 1f, 3f, 3.5f, 2.5f, 1f, 0f, -1f),
    "Rock" to listOf(4f, 3f, 1.5f, -0.5f, -1f, 0f, 1.5f, 3f, 3.5f, 4f),
    "Pop" to listOf(-1f, 1f, 2.5f, 3.5f, 3f, 0.5f, -0.5f, -1f, 0f, 0.5f),
    "Jazz" to listOf(3f, 2f, 1f, 1.5f, -1f, -1f, 0f, 1f, 2f, 3f),
    "Classical" to listOf(3f, 2f, 1f, 0f, 0f, 0f, -0.5f, -0.5f, 1f, 2f),
    "Electronic" to listOf(5f, 4f, 1.5f, 0f, -1.5f, 1f, 0f, 1f, 3.5f, 4.5f),
    "Loudness" to listOf(5f, 3f, 0.5f, 0f, -0.5f, 0f, 0f, 0.5f, 2.5f, 4f),
)

/** The name of a curve that's no preset's (the user's own). */
const val EQ_CUSTOM = "Custom"

/** The ReplayGain pre-amp's limit, dB. */
const val MAX_PREAMP_DB = 15f

/** The longest crossfade, as on Windows. */
const val MAX_CROSSFADE_MS = 12_000

@Serializable
data class EqSettings(
    val enabled: Boolean = false,
    /** The preset the gains came from ([EQ_CUSTOM] once edited). */
    val preset: String = "Flat",
    /** Gain per band in dB, one per [EQ_BANDS] entry. */
    val gainsDb: List<Float> = List(EQ_BANDS.size) { 0f },
) {
    /** On and not flat: the samples will change. */
    val active: Boolean get() = enabled && gainsDb.any { abs(it) >= 0.05f }

    /** Headroom for the boosts, so the sum stays below full scale. */
    val preampDb: Float get() = -gainsDb.fold(0f, ::maxOf)

    /** A preset's curve (the settings keep its name). */
    fun withPreset(name: String): EqSettings = EQ_PRESETS.firstOrNull { it.first == name }?.let { copy(preset = name, gainsDb = it.second) } ?: this

    /** One band moved: the curve is now the user's own, unless it matches a preset again. */
    fun withBand(band: Int, db: Float): EqSettings {
        val gains = EQ_BANDS.indices.map { i -> if (i == band) clampDb(db) else gainsDb.getOrElse(i) { 0f } }
        val match = EQ_PRESETS.firstOrNull { (_, curve) -> curve.indices.all { abs(curve[it] - gains[it]) < 0.05f } }
        return copy(gainsDb = gains, preset = match?.first ?: EQ_CUSTOM)
    }

    /** One finite gain per band, within ±[EQ_MAX_DB] (preferences can be edited by hand). */
    fun normalized(): EqSettings =
        copy(gainsDb = EQ_BANDS.indices.map { i -> gainsDb.getOrNull(i)?.takeIf(Float::isFinite)?.coerceIn(-EQ_MAX_DB, EQ_MAX_DB) ?: 0f })
}

private fun clampDb(db: Float) = (round(db * 2) / 2).coerceIn(-EQ_MAX_DB, EQ_MAX_DB)

@Serializable
enum class ReplayGainMode {
    Off,

    /** Each track at the same loudness. */
    Track,

    /** Albums at the same loudness, keeping the differences between their tracks. */
    Album,
}

/** How Pixeltone treats the sound on Android's output, as on Windows (Settings → Sound). */
@Serializable
data class Sound(
    val eq: EqSettings = EqSettings(),
    val replayGain: ReplayGainMode = ReplayGainMode.Off,
    val replayGainPreampDb: Float = 0f,
    /** 0: one track follows the other gaplessly. */
    val crossfadeMs: Int = 0,
    /** Crossfade between an album's own tracks too (else albums stay gapless). */
    val crossfadeAlbums: Boolean = false,
) {
    fun normalized(): Sound = copy(
        eq = eq.normalized(),
        replayGainPreampDb = replayGainPreampDb.takeIf(Float::isFinite)?.coerceIn(-MAX_PREAMP_DB, MAX_PREAMP_DB) ?: 0f,
        crossfadeMs = crossfadeMs.coerceIn(0, MAX_CROSSFADE_MS),
    )
}

/** The sound settings, kept in the app's preferences; [playingTags] are those of the phone's track that plays. */
class SoundSettings(context: Context) {
    private val prefs = context.getSharedPreferences("sound", Context.MODE_PRIVATE)
    private val json = Json { ignoreUnknownKeys = true }
    private val current = MutableStateFlow(load())

    val sound: StateFlow<Sound> = current.asStateFlow()

    /** The ReplayGain tags of the phone's track on Android's output, as the sound processing met them. */
    val playingTags = MutableStateFlow(ReplayGainTags())

    fun update(change: (Sound) -> Sound) {
        val next = change(current.value).normalized()
        current.value = next
        prefs.edit { putString(KEY, json.encodeToString(Sound.serializer(), next)) }
    }

    private fun load(): Sound =
        prefs.getString(KEY, null)?.let { runCatching { json.decodeFromString(Sound.serializer(), it) }.getOrNull() }?.normalized() ?: Sound()

    private companion object {
        const val KEY = "sound"
    }
}
