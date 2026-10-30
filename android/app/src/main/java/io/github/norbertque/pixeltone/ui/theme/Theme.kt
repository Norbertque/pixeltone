package io.github.norbertque.pixeltone.ui.theme

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.Color

/**
 * Pixeltone's theme: colours from [seed] (the playing album's cover, else Pixeltone's lilac) in
 * [style], gliding from one cover's to the next as on Windows; Google Sans Flex type.
 */
@Composable
fun PixeltoneTheme(
    dark: Boolean = isSystemInDarkTheme(),
    seed: Int? = null,
    style: PaletteStyle = PaletteStyle.TonalSpot,
    contrast: Double = 0.0,
    content: @Composable () -> Unit,
) {
    val target = remember(seed, dark, style, contrast) { colorSchemeFor(seed ?: DEFAULT_SEED, dark, style, contrast) }
    // The shape scale is Material's own, as on Windows (4, 8, 12, 16, 28 px).
    MaterialTheme(colorScheme = target.animated(), typography = PixeltoneTypography, content = content)
}

/** Every role eases to its new colour over 700 ms, like the Windows theme. */
@Composable
private fun ColorScheme.animated(): ColorScheme {
    @Composable
    fun Color.eased(): Color {
        val color by animateColorAsState(this, tween(700), label = "theme")
        return color
    }
    return copy(
        primary = primary.eased(), onPrimary = onPrimary.eased(),
        primaryContainer = primaryContainer.eased(), onPrimaryContainer = onPrimaryContainer.eased(),
        inversePrimary = inversePrimary.eased(),
        secondary = secondary.eased(), onSecondary = onSecondary.eased(),
        secondaryContainer = secondaryContainer.eased(), onSecondaryContainer = onSecondaryContainer.eased(),
        tertiary = tertiary.eased(), onTertiary = onTertiary.eased(),
        tertiaryContainer = tertiaryContainer.eased(), onTertiaryContainer = onTertiaryContainer.eased(),
        background = background.eased(), onBackground = onBackground.eased(),
        surface = surface.eased(), onSurface = onSurface.eased(),
        surfaceVariant = surfaceVariant.eased(), onSurfaceVariant = onSurfaceVariant.eased(),
        surfaceTint = surfaceTint.eased(),
        inverseSurface = inverseSurface.eased(), inverseOnSurface = inverseOnSurface.eased(),
        outline = outline.eased(), outlineVariant = outlineVariant.eased(),
        surfaceBright = surfaceBright.eased(), surfaceDim = surfaceDim.eased(),
        surfaceContainer = surfaceContainer.eased(), surfaceContainerHigh = surfaceContainerHigh.eased(),
        surfaceContainerHighest = surfaceContainerHighest.eased(), surfaceContainerLow = surfaceContainerLow.eased(),
        surfaceContainerLowest = surfaceContainerLowest.eased(),
    )
}
