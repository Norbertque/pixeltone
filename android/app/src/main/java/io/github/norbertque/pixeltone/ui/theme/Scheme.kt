package io.github.norbertque.pixeltone.ui.theme

import android.graphics.Bitmap
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color
import androidx.core.graphics.scale
import com.materialkolor.dynamiccolor.ColorSpec
import com.materialkolor.dynamiccolor.DynamicColor
import com.materialkolor.dynamiccolor.MaterialDynamicColors
import com.materialkolor.hct.Hct
import com.materialkolor.quantize.QuantizerCelebi
import com.materialkolor.scheme.DynamicScheme
import com.materialkolor.scheme.SchemeTonalSpot
import com.materialkolor.score.Score

/** Pixeltone's own colour when nothing plays: "Lilac", as on Windows. */
const val DEFAULT_SEED: Int = 0xFF7E57C2.toInt()

/**
 * The colour scheme for a seed colour, built like the Windows app's (`ui/src/lib/theme/scheme.ts`):
 * tonal spot, the 2025 colour spec.
 */
fun colorSchemeFor(seed: Int, dark: Boolean): ColorScheme {
    val scheme = SchemeTonalSpot(Hct.fromInt(seed), dark, 0.0, ColorSpec.SpecVersion.SPEC_2025, DynamicScheme.Platform.PHONE)
    val roles = MaterialDynamicColors()
    fun c(color: DynamicColor) = Color(color.getArgb(scheme))
    val primary = c(roles.primary())
    val onPrimary = c(roles.onPrimary())
    val primaryContainer = c(roles.primaryContainer())
    val onPrimaryContainer = c(roles.onPrimaryContainer())
    val inversePrimary = c(roles.inversePrimary())
    val secondary = c(roles.secondary())
    val onSecondary = c(roles.onSecondary())
    val secondaryContainer = c(roles.secondaryContainer())
    val onSecondaryContainer = c(roles.onSecondaryContainer())
    val tertiary = c(roles.tertiary())
    val onTertiary = c(roles.onTertiary())
    val tertiaryContainer = c(roles.tertiaryContainer())
    val onTertiaryContainer = c(roles.onTertiaryContainer())
    val background = c(roles.background())
    val onBackground = c(roles.onBackground())
    val surface = c(roles.surface())
    val onSurface = c(roles.onSurface())
    val surfaceVariant = c(roles.surfaceVariant())
    val onSurfaceVariant = c(roles.onSurfaceVariant())
    val surfaceTint = c(roles.surfaceTint())
    val inverseSurface = c(roles.inverseSurface())
    val inverseOnSurface = c(roles.inverseOnSurface())
    val error = c(roles.error())
    val onError = c(roles.onError())
    val errorContainer = c(roles.errorContainer())
    val onErrorContainer = c(roles.onErrorContainer())
    val outline = c(roles.outline())
    val outlineVariant = c(roles.outlineVariant())
    val scrim = c(roles.scrim())
    val surfaceBright = c(roles.surfaceBright())
    val surfaceContainer = c(roles.surfaceContainer())
    val surfaceContainerHigh = c(roles.surfaceContainerHigh())
    val surfaceContainerHighest = c(roles.surfaceContainerHighest())
    val surfaceContainerLow = c(roles.surfaceContainerLow())
    val surfaceContainerLowest = c(roles.surfaceContainerLowest())
    val surfaceDim = c(roles.surfaceDim())
    return if (dark) {
        darkColorScheme(
            primary = primary, onPrimary = onPrimary, primaryContainer = primaryContainer,
            onPrimaryContainer = onPrimaryContainer, inversePrimary = inversePrimary,
            secondary = secondary, onSecondary = onSecondary, secondaryContainer = secondaryContainer,
            onSecondaryContainer = onSecondaryContainer, tertiary = tertiary, onTertiary = onTertiary,
            tertiaryContainer = tertiaryContainer, onTertiaryContainer = onTertiaryContainer,
            background = background, onBackground = onBackground, surface = surface, onSurface = onSurface,
            surfaceVariant = surfaceVariant, onSurfaceVariant = onSurfaceVariant, surfaceTint = surfaceTint,
            inverseSurface = inverseSurface, inverseOnSurface = inverseOnSurface, error = error, onError = onError,
            errorContainer = errorContainer, onErrorContainer = onErrorContainer, outline = outline,
            outlineVariant = outlineVariant, scrim = scrim, surfaceBright = surfaceBright,
            surfaceContainer = surfaceContainer, surfaceContainerHigh = surfaceContainerHigh,
            surfaceContainerHighest = surfaceContainerHighest, surfaceContainerLow = surfaceContainerLow,
            surfaceContainerLowest = surfaceContainerLowest, surfaceDim = surfaceDim,
        )
    } else {
        lightColorScheme(
            primary = primary, onPrimary = onPrimary, primaryContainer = primaryContainer,
            onPrimaryContainer = onPrimaryContainer, inversePrimary = inversePrimary,
            secondary = secondary, onSecondary = onSecondary, secondaryContainer = secondaryContainer,
            onSecondaryContainer = onSecondaryContainer, tertiary = tertiary, onTertiary = onTertiary,
            tertiaryContainer = tertiaryContainer, onTertiaryContainer = onTertiaryContainer,
            background = background, onBackground = onBackground, surface = surface, onSurface = onSurface,
            surfaceVariant = surfaceVariant, onSurfaceVariant = onSurfaceVariant, surfaceTint = surfaceTint,
            inverseSurface = inverseSurface, inverseOnSurface = inverseOnSurface, error = error, onError = onError,
            errorContainer = errorContainer, onErrorContainer = onErrorContainer, outline = outline,
            outlineVariant = outlineVariant, scrim = scrim, surfaceBright = surfaceBright,
            surfaceContainer = surfaceContainer, surfaceContainerHigh = surfaceContainerHigh,
            surfaceContainerHighest = surfaceContainerHighest, surfaceContainerLow = surfaceContainerLow,
            surfaceContainerLowest = surfaceContainerLowest, surfaceDim = surfaceDim,
        )
    }
}

/**
 * The seed colour of a picture, the way Android picks wallpaper colours and the Windows app
 * its theme: shrunk to 112 px, quantised to 128 colours, then scored. Null for a picture with
 * nothing colourful enough.
 */
fun seedOf(picture: Bitmap): Int? {
    val small = picture.scale(SEED_SIZE, SEED_SIZE)
    val pixels = IntArray(SEED_SIZE * SEED_SIZE)
    small.getPixels(pixels, 0, SEED_SIZE, 0, 0, SEED_SIZE, SEED_SIZE)
    val opaque = pixels.filter { (it ushr 24) == 0xFF }.toIntArray()
    if (opaque.isEmpty()) return null
    val colors = QuantizerCelebi.quantize(opaque, 128)
    return Score.score(colors, 1, null, true).firstOrNull()
}

private const val SEED_SIZE = 112
