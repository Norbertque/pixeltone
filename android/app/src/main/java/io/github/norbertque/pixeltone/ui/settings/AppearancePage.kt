package io.github.norbertque.pixeltone.ui.settings

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.materialkolor.dynamiccolor.MaterialDynamicColors
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.ui.theme.Appearance
import io.github.norbertque.pixeltone.ui.theme.ColorSource
import io.github.norbertque.pixeltone.ui.theme.PaletteStyle
import io.github.norbertque.pixeltone.ui.theme.SEED_PRESETS
import io.github.norbertque.pixeltone.ui.theme.ThemeMode
import io.github.norbertque.pixeltone.ui.theme.dynamicScheme

@Composable
fun AppearanceScreen(app: PixeltoneApp, onBack: () -> Unit) {
    val look by app.appearance.appearance.collectAsStateWithLifecycle()
    AppearanceContent(look, onChange = app.appearance::update, onBack = onBack)
}

/** Light or dark, where the colours come from, the palette's style and contrast, as on Windows. */
@Composable
fun AppearanceContent(look: Appearance, onChange: ((Appearance) -> Appearance) -> Unit, onBack: () -> Unit) {
    SettingsScaffold(stringResource(R.string.appearance), onBack) {
        item {
            ChoiceRow(
                stringResource(R.string.theme),
                stringResource(R.string.theme_text),
                listOf(
                    ThemeMode.System to stringResource(R.string.theme_system),
                    ThemeMode.Light to stringResource(R.string.theme_light),
                    ThemeMode.Dark to stringResource(R.string.theme_dark),
                ),
                look.mode,
            ) { mode -> onChange { it.copy(mode = mode) } }
        }
        item {
            ChoiceRow(
                stringResource(R.string.colour_source),
                stringResource(R.string.colour_source_text),
                listOf(ColorSource.AlbumArt to stringResource(R.string.album_art), ColorSource.Custom to stringResource(R.string.custom)),
                look.source,
            ) { source -> onChange { it.copy(source = source) } }
        }
        item { Swatches(look) { seed -> onChange { it.copy(seed = seed) } } }
        item { Styles(look) { style -> onChange { it.copy(style = style) } } }
        item {
            ChoiceRow(
                stringResource(R.string.contrast),
                null,
                listOf(0.0 to stringResource(R.string.contrast_standard), 0.5 to stringResource(R.string.contrast_medium), 1.0 to stringResource(R.string.contrast_high)),
                look.contrast,
            ) { contrast -> onChange { it.copy(contrast = contrast) } }
        }
    }
}

/** Windows' colours to choose from, as circles; the chosen one has a tick. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun Swatches(look: Appearance, onPick: (Int) -> Unit) {
    val fromCovers = look.source == ColorSource.AlbumArt
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp)) {
        Text(stringResource(if (fromCovers) R.string.fallback_colour else R.string.seed_colour), style = MaterialTheme.typography.bodyLarge)
        Text(
            stringResource(if (fromCovers) R.string.fallback_colour_text else R.string.seed_colour_text),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        FlowRow(Modifier.padding(top = 12.dp), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            SEED_PRESETS.forEach { (name, seed) ->
                val chosen = seed == look.seed
                Box(
                    Modifier
                        .size(48.dp)
                        .clip(CircleShape)
                        .background(Color(seed))
                        .selectable(selected = chosen, role = Role.RadioButton) { onPick(seed) }
                        .semantics { contentDescription = name },
                    contentAlignment = Alignment.Center,
                ) {
                    if (chosen) Icon(Icons.Default.Check, null, tint = if (Color(seed).luminance() > 0.5f) Color.Black else Color.White)
                }
            }
        }
    }
}

/** The palette styles side by side, each with three of the colours it gives (as on Windows). */
@Composable
private fun Styles(look: Appearance, onPick: (PaletteStyle) -> Unit) {
    val dark = MaterialTheme.colorScheme.surface.luminance() < 0.5f
    // What plays gives the colours now, so the previews start from the theme's own primary.
    val seed = MaterialTheme.colorScheme.primary.toArgb()
    Column(Modifier.fillMaxWidth().padding(vertical = 12.dp)) {
        Text(stringResource(R.string.palette_style), Modifier.padding(horizontal = 16.dp), style = MaterialTheme.typography.bodyLarge)
        Text(
            stringResource(R.string.palette_style_text),
            Modifier.padding(horizontal = 16.dp),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Row(
            Modifier.horizontalScroll(rememberScrollState()).padding(horizontal = 16.dp).padding(top = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            PaletteStyle.entries.forEach { style ->
                val chosen = style == look.style
                val colors = remember(style, seed, dark) {
                    val scheme = dynamicScheme(seed, dark, style)
                    val roles = MaterialDynamicColors()
                    listOf(roles.primary(), roles.secondaryContainer(), roles.tertiary()).map { Color(it.getArgb(scheme)) }
                }
                Surface(
                    selected = chosen,
                    onClick = { onPick(style) },
                    modifier = Modifier.width(104.dp),
                    shape = MaterialTheme.shapes.large,
                    color = if (chosen) MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.surfaceContainer,
                    border = if (chosen) BorderStroke(2.dp, MaterialTheme.colorScheme.primary) else null,
                ) {
                    Column(Modifier.padding(vertical = 12.dp, horizontal = 8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                        Row(horizontalArrangement = Arrangement.spacedBy((-6).dp)) {
                            colors.forEach { Box(Modifier.size(24.dp).clip(CircleShape).background(it)) }
                        }
                        Text(
                            stringResource(styleName(style)),
                            Modifier.padding(top = 8.dp),
                            style = MaterialTheme.typography.labelLarge,
                            textAlign = TextAlign.Center,
                            maxLines = 1,
                        )
                    }
                }
            }
        }
    }
}

private fun styleName(style: PaletteStyle): Int =
    when (style) {
        PaletteStyle.TonalSpot -> R.string.style_tonal_spot
        PaletteStyle.Expressive -> R.string.style_expressive
        PaletteStyle.Vibrant -> R.string.style_vibrant
        PaletteStyle.Fidelity -> R.string.style_fidelity
        PaletteStyle.Content -> R.string.style_content
        PaletteStyle.Rainbow -> R.string.style_rainbow
        PaletteStyle.FruitSalad -> R.string.style_fruit_salad
        PaletteStyle.Neutral -> R.string.style_neutral
        PaletteStyle.Monochrome -> R.string.style_monochrome
    }
