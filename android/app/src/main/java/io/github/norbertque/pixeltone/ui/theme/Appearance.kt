package io.github.norbertque.pixeltone.ui.theme

import android.content.Context
import androidx.core.content.edit
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Follow the phone, or always light or dark. */
enum class ThemeMode { System, Light, Dark }

/** Where the colours come from: the cover of what plays, or the colour chosen in Settings. */
enum class ColorSource { AlbumArt, Custom }

/** Material You's palette styles, as on Windows (`PALETTE_STYLES`). */
enum class PaletteStyle { TonalSpot, Expressive, Vibrant, Fidelity, Content, Rainbow, FruitSalad, Neutral, Monochrome }

/** Colours to choose from, as on Windows (`SEED_PRESETS`); Lilac is Pixeltone's own. */
val SEED_PRESETS: List<Pair<String, Int>> = listOf(
    "Lilac" to DEFAULT_SEED,
    "Ocean" to 0xFF1E6FD9.toInt(),
    "Teal" to 0xFF00897B.toInt(),
    "Moss" to 0xFF5B8C2A.toInt(),
    "Amber" to 0xFFE0A100.toInt(),
    "Coral" to 0xFFF4511E.toInt(),
    "Rose" to 0xFFD81B60.toInt(),
    "Slate" to 0xFF546E7A.toInt(),
)

/** How Pixeltone looks. */
data class Appearance(
    val mode: ThemeMode = ThemeMode.System,
    val source: ColorSource = ColorSource.AlbumArt,
    /** The colour chosen: every colour's seed with [ColorSource.Custom], else while no cover gives one. */
    val seed: Int = DEFAULT_SEED,
    val style: PaletteStyle = PaletteStyle.TonalSpot,
    /** 0 standard, 0.5 medium, 1 high. */
    val contrast: Double = 0.0,
)

/** The look chosen in Settings, kept in the app's preferences. */
class AppearanceSettings(context: Context) {
    private val prefs = context.getSharedPreferences("appearance", Context.MODE_PRIVATE)
    private val current = MutableStateFlow(load())

    val appearance: StateFlow<Appearance> = current.asStateFlow()

    fun update(change: (Appearance) -> Appearance) {
        val look = change(current.value)
        current.value = look
        prefs.edit {
            putString(MODE, look.mode.name)
            putString(SOURCE, look.source.name)
            putInt(SEED, look.seed)
            putString(STYLE, look.style.name)
            putFloat(CONTRAST, look.contrast.toFloat())
        }
    }

    private fun load() = Appearance(
        mode = enumOf(prefs.getString(MODE, null), ThemeMode.System),
        source = enumOf(prefs.getString(SOURCE, null), ColorSource.AlbumArt),
        seed = prefs.getInt(SEED, DEFAULT_SEED),
        style = enumOf(prefs.getString(STYLE, null), PaletteStyle.TonalSpot),
        contrast = prefs.getFloat(CONTRAST, 0f).toDouble(),
    )

    private companion object {
        const val MODE = "mode"
        const val SOURCE = "source"
        const val SEED = "seed"
        const val STYLE = "style"
        const val CONTRAST = "contrast"

        inline fun <reified T : Enum<T>> enumOf(name: String?, default: T): T = enumValues<T>().firstOrNull { it.name == name } ?: default
    }
}
