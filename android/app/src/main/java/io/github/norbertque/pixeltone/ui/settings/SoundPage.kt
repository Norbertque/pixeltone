package io.github.norbertque.pixeltone.ui.settings

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.setProgress
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.playback.EQ_BANDS
import io.github.norbertque.pixeltone.playback.EQ_CUSTOM
import io.github.norbertque.pixeltone.playback.EQ_MAX_DB
import io.github.norbertque.pixeltone.playback.EQ_PRESETS
import io.github.norbertque.pixeltone.playback.EqSettings
import io.github.norbertque.pixeltone.playback.MAX_CROSSFADE_MS
import io.github.norbertque.pixeltone.playback.MAX_PREAMP_DB
import io.github.norbertque.pixeltone.playback.ReplayGainMode
import io.github.norbertque.pixeltone.playback.Sound
import java.util.Locale
import kotlin.math.abs
import kotlin.math.round

@Composable
fun SoundScreen(app: PixeltoneApp, onBack: () -> Unit) {
    val sound by app.sound.sound.collectAsStateWithLifecycle()
    val usb by app.usbSettings.exclusiveEnabled.collectAsStateWithLifecycle()
    SoundContent(sound, onChange = app.sound::update, usbExclusive = usb, onBack = onBack)
}

/** The equaliser and ReplayGain, as on Windows (Settings → Sound). */
@Composable
fun SoundContent(sound: Sound, onChange: ((Sound) -> Sound) -> Unit, usbExclusive: Boolean, onBack: () -> Unit) {
    SettingsScaffold(stringResource(R.string.sound), onBack) {
        if (usbExclusive) {
            item {
                Text(
                    stringResource(R.string.sound_usb_note),
                    Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
            }
        }
        item {
            SwitchRow(stringResource(R.string.equaliser), stringResource(R.string.equaliser_text), sound.eq.enabled) { on ->
                onChange { it.copy(eq = it.eq.copy(enabled = on)) }
            }
        }
        item { EqualiserBands(sound.eq) { eq -> onChange { it.copy(eq = eq) } } }
        item {
            ChoiceRow(
                stringResource(R.string.replay_gain),
                stringResource(R.string.replay_gain_text),
                listOf(
                    ReplayGainMode.Off to stringResource(R.string.off),
                    ReplayGainMode.Track to stringResource(R.string.replay_gain_track),
                    ReplayGainMode.Album to stringResource(R.string.replay_gain_album),
                ),
                sound.replayGain,
            ) { mode -> onChange { it.copy(replayGain = mode) } }
        }
        if (sound.replayGain != ReplayGainMode.Off) {
            item {
                SliderRow(
                    stringResource(R.string.preamp),
                    value = sound.replayGainPreampDb,
                    range = -MAX_PREAMP_DB..MAX_PREAMP_DB,
                    steps = 0,
                    shown = formatSignedDb(sound.replayGainPreampDb),
                ) { db -> onChange { it.copy(replayGainPreampDb = round(db * 2) / 2) } }
            }
        }
        item {
            val seconds = sound.crossfadeMs / 1000f
            SliderRow(
                stringResource(R.string.crossfade),
                text = stringResource(R.string.crossfade_text),
                value = seconds,
                range = 0f..MAX_CROSSFADE_MS / 1000f,
                steps = 0,
                shown = if (sound.crossfadeMs == 0) {
                    stringResource(R.string.off)
                } else {
                    stringResource(R.string.seconds_value, String.format(Locale.ROOT, "%.1f", seconds).removeSuffix(".0"))
                },
            ) { s -> onChange { it.copy(crossfadeMs = (round(s * 2) / 2 * 1000).toInt()) } }
        }
        if (sound.crossfadeMs > 0) {
            item {
                SwitchRow(stringResource(R.string.crossfade_albums), stringResource(R.string.crossfade_albums_text), sound.crossfadeAlbums) { on ->
                    onChange { it.copy(crossfadeAlbums = on) }
                }
            }
        }
    }
}

/** "+3.5 dB", "−6 dB", "0 dB". */
internal fun formatSignedDb(db: Float): String = formatBandDb(db) + " dB"

/** A setting on a slider, its value beside it. */
@Composable
internal fun SliderRow(
    title: String,
    text: String? = null,
    value: Float,
    range: ClosedFloatingPointRange<Float>,
    steps: Int,
    shown: String,
    onChange: (Float) -> Unit,
) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(title, Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge)
            Text(shown, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
        }
        if (text != null) Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Slider(value = value, onValueChange = onChange, valueRange = range, steps = steps)
    }
}

/** The preset menu and a slider per band; dimmed while the equaliser is off (a touch turns it on). */
@Composable
private fun EqualiserBands(eq: EqSettings, onChange: (EqSettings) -> Unit) {
    var choosing by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp).alpha(if (eq.enabled) 1f else 0.6f)) {
        Box {
            OutlinedButton(onClick = { choosing = true }) {
                Text(stringResource(R.string.preset_named, if (eq.preset == EQ_CUSTOM) stringResource(R.string.preset_custom) else eq.preset))
                Icon(Icons.Default.ArrowDropDown, null)
            }
            DropdownMenu(expanded = choosing, onDismissRequest = { choosing = false }) {
                EQ_PRESETS.forEach { (name, _) ->
                    DropdownMenuItem(
                        text = { Text(name) },
                        onClick = {
                            choosing = false
                            onChange(eq.copy(enabled = true).withPreset(name))
                        },
                    )
                }
            }
        }
        Row(Modifier.fillMaxWidth().padding(top = 12.dp), horizontalArrangement = Arrangement.SpaceBetween) {
            EQ_BANDS.forEachIndexed { band, hz ->
                val db = eq.gainsDb.getOrElse(band) { 0f }
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(formatBandDb(db), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    BandSlider(db, label = "${bandName(hz)} Hz") { onChange(eq.copy(enabled = true).withBand(band, it)) }
                    Text(bandName(hz), style = MaterialTheme.typography.labelSmall, textAlign = TextAlign.Center)
                }
            }
        }
        Text(
            stringResource(R.string.equaliser_reset_hint),
            Modifier.padding(vertical = 8.dp),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

private fun bandName(hz: Int) = if (hz >= 1000) "${hz / 1000}k" else hz.toString()

/** "+3.5", "−6", "0", in half dB. */
private fun formatBandDb(db: Float): String {
    val half = round(db * 2) / 2
    if (abs(half) < 0.05f) return "0"
    return (if (half > 0) "+" else "−") + String.format(Locale.ROOT, "%.1f", abs(half)).removeSuffix(".0")
}

/**
 * One band's gain, up and down: a track with a dot, filled from 0 dB to the gain. A drag or a
 * tap sets it in half dB, a double tap puts it back to 0.
 */
@Composable
private fun BandSlider(db: Float, label: String, onChange: (Float) -> Unit) {
    val primary = MaterialTheme.colorScheme.primary
    val track = MaterialTheme.colorScheme.secondaryContainer
    val pad = 10.dp
    fun at(y: Float, height: Float, padPx: Float): Float {
        val fraction = 1f - ((y - padPx) / (height - 2 * padPx).coerceAtLeast(1f)).coerceIn(0f, 1f)
        return round((fraction * 2 * EQ_MAX_DB - EQ_MAX_DB) * 2) / 2
    }
    Canvas(
        Modifier
            .width(28.dp)
            .height(168.dp)
            .semantics {
                contentDescription = label
                stateDescription = formatBandDb(db) + " dB"
                progressBarRangeInfo = ProgressBarRangeInfo(db, -EQ_MAX_DB..EQ_MAX_DB, steps = (EQ_MAX_DB * 4).toInt() - 1)
                setProgress { target ->
                    onChange(round(target * 2) / 2)
                    true
                }
            }
            .pointerInput(Unit) {
                val padPx = pad.toPx()
                detectTapGestures(onDoubleTap = { onChange(0f) }, onTap = { onChange(at(it.y, size.height.toFloat(), padPx)) })
            }
            .pointerInput(Unit) {
                val padPx = pad.toPx()
                detectVerticalDragGestures { change, _ ->
                    change.consume()
                    onChange(at(change.position.y, size.height.toFloat(), padPx))
                }
            },
    ) {
        val padPx = pad.toPx()
        val x = size.width / 2
        val top = padPx
        val bottom = size.height - padPx
        fun y(value: Float) = bottom - (value + EQ_MAX_DB) / (2 * EQ_MAX_DB) * (bottom - top)
        val stroke = 4.dp.toPx()
        drawLine(track, Offset(x, top), Offset(x, bottom), stroke, StrokeCap.Round)
        drawLine(track.copy(alpha = 1f), Offset(x - 6.dp.toPx(), y(0f)), Offset(x + 6.dp.toPx(), y(0f)), 2.dp.toPx(), StrokeCap.Round)
        drawLine(primary, Offset(x, y(0f)), Offset(x, y(db)), stroke, StrokeCap.Round)
        drawCircle(primary, radius = 8.dp.toPx(), center = Offset(x, y(db)))
    }
}
