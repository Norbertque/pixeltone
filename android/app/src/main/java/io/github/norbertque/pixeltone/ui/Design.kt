package io.github.norbertque.pixeltone.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.setProgress
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.exp
import kotlin.math.sin

/**
 * The seek bar of Pixeltone on Windows (`WavyProgress.svelte`): the played part is a wave that
 * moves while the track plays and flattens when it pauses; a short bar marks where it is.
 * Dragging or tapping seeks ([onSeek] gets 0..1 when the finger lifts).
 */
@Composable
fun WavySeekBar(
    value: Float,
    playing: Boolean,
    onSeek: (Float) -> Unit,
    label: String,
    modifier: Modifier = Modifier,
    thickness: Dp = 6.dp,
    amplitude: Dp = 4.5.dp,
    wavelength: Dp = 42.dp,
    height: Dp = 44.dp,
) {
    var held by remember { mutableStateOf<Float?>(null) }
    var amp by remember { mutableFloatStateOf(0f) }
    var phase by remember { mutableFloatStateOf(0f) }
    val waving = playing && held == null
    // The wave eases in and out (~150 ms, critically damped) and travels while it shows.
    LaunchedEffect(waving) {
        var last = 0L
        while (true) {
            val now = withFrameNanos { it }
            val dt = if (last == 0L) 0f else ((now - last) / 1e9f).coerceAtMost(0.05f)
            last = now
            val target = if (waving) 1f else 0f
            amp += (target - amp) * (1 - exp(-dt * 14f))
            if (kotlin.math.abs(target - amp) < 0.01f) amp = target
            if (amp > 0f) phase += dt * 7f
            if (!waving && amp == 0f) break
        }
    }
    val shown = (held ?: value).coerceIn(0f, 1f)
    val primary = MaterialTheme.colorScheme.primary
    val track = MaterialTheme.colorScheme.secondaryContainer
    val thumbColor = if (held != null) MaterialTheme.colorScheme.onPrimaryContainer else primary
    Canvas(
        modifier
            .fillMaxWidth()
            .height(height)
            .semantics {
                contentDescription = label
                progressBarRangeInfo = ProgressBarRangeInfo(shown, 0f..1f)
                setProgress { target ->
                    onSeek(target.coerceIn(0f, 1f))
                    true
                }
            }
            .pointerInput(Unit) {
                val pad = thickness.toPx() / 2 + amplitude.toPx()
                fun at(x: Float) = ((x - pad) / (size.width - pad * 2).coerceAtLeast(1f)).coerceIn(0f, 1f)
                detectTapGestures { onSeek(at(it.x)) }
            }
            .pointerInput(Unit) {
                val pad = thickness.toPx() / 2 + amplitude.toPx()
                fun at(x: Float) = ((x - pad) / (size.width - pad * 2).coerceAtLeast(1f)).coerceIn(0f, 1f)
                detectHorizontalDragGestures(
                    onDragStart = { held = at(it.x) },
                    onDragEnd = {
                        held?.let(onSeek)
                        held = null
                    },
                    onDragCancel = { held = null },
                    onHorizontalDrag = { change, _ ->
                        change.consume()
                        held = at(change.position.x)
                    },
                )
            },
    ) {
        val stroke = thickness.toPx()
        val ampPx = amplitude.toPx() * amp
        val pad = stroke / 2 + amplitude.toPx()
        val mid = size.height / 2
        val span = (size.width - pad * 2).coerceAtLeast(0f)
        val x = pad + span * shown
        val gap = 6.dp.toPx()
        val thumbWidth = 4.dp.toPx()
        val end = (x - gap - thumbWidth / 2).coerceAtLeast(pad)
        if (end > pad) {
            val k = (2 * PI / wavelength.toPx()).toFloat()
            val wave = Path()
            var px = pad
            wave.moveTo(px, mid + ampPx * sin(px * k - phase))
            while (px < end) {
                px = (px + 2f).coerceAtMost(end)
                wave.lineTo(px, mid + ampPx * sin(px * k - phase))
            }
            drawPath(wave, primary, style = Stroke(stroke, cap = StrokeCap.Round))
        }
        val start = (x + gap + thumbWidth / 2).coerceAtMost(size.width - pad)
        if (start < size.width - pad) {
            drawLine(track, Offset(start, mid), Offset(size.width - pad, mid), stroke, StrokeCap.Round)
        }
        val thumbHeight = size.height * 0.72f
        drawRoundRect(
            thumbColor,
            topLeft = Offset(x - thumbWidth / 2, mid - thumbHeight / 2),
            size = Size(thumbWidth, thumbHeight),
            cornerRadius = CornerRadius(thumbWidth / 2),
        )
    }
}

/**
 * A circle with soft scallops ("cookie", as Material 3 Expressive and Pixeltone's logo on
 * Windows draw them: `shapes.ts`); [depth] 0 is a plain circle.
 */
class CookieShape(private val lobes: Int = 9, private val depth: Float = 0.07f, private val rotation: Float = 0f) : Shape {
    override fun createOutline(size: Size, layoutDirection: LayoutDirection, density: Density): Outline {
        val c = minOf(size.width, size.height) / 2
        val base = c / (1 + depth)
        val path = Path()
        val points = 144
        for (i in 0 until points) {
            val a = i.toFloat() / points * 2 * PI.toFloat() + rotation
            val r = base * (1 + depth * cos(lobes * (a - rotation)))
            val px = size.width / 2 + r * cos(a)
            val py = size.height / 2 + r * sin(a)
            if (i == 0) path.moveTo(px, py) else path.lineTo(px, py)
        }
        path.close()
        return Outline.Generic(path)
    }
}
