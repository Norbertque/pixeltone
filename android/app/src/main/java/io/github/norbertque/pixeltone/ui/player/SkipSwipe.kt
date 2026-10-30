package io.github.norbertque.pixeltone.ui.player

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import kotlin.math.abs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * A sideways swipe that skips, on Now Playing's cover and on the mini player. What shows follows
 * the finger; let go past a quarter of the width, it flies off and the next track (to the left) or
 * the previous one (to the right) slides in from the other side. Short of that, it springs back.
 */
@Stable
class SkipSwipe internal constructor(
    private val scope: CoroutineScope,
    private val onNext: () -> Unit,
    private val onPrevious: () -> Unit,
) {
    internal val offset = Animatable(0f)
    private var width = 1f

    /** Where it flew: -1 left (next), 1 right (previous), 0 nowhere. */
    private var flown = 0

    internal fun start(width: Float) {
        this.width = width.coerceAtLeast(1f)
    }

    internal fun drag(dx: Float) {
        scope.launch { offset.snapTo(offset.value + dx) }
    }

    internal fun release() {
        scope.launch {
            val direction = when {
                offset.value < -width * SKIP_AT -> -1
                offset.value > width * SKIP_AT -> 1
                else -> 0
            }
            if (direction == 0) {
                offset.animateTo(0f)
                return@launch
            }
            flown = direction
            offset.animateTo(direction * width, tween(FLY_MS))
            if (direction < 0) onNext() else onPrevious()
            // No other track came (the queue's end, or previous started this one over): back in place.
            delay(WAIT_MS)
            if (flown != 0) {
                flown = 0
                offset.animateTo(0f)
            }
        }
    }

    internal fun cancel() {
        scope.launch { offset.animateTo(0f) }
    }

    /** Another track shows: after a swipe, it comes in from the other side. */
    internal suspend fun arrived() {
        val direction = flown
        if (direction == 0) return
        flown = 0
        offset.snapTo(-direction * width)
        offset.animateTo(0f, spring(stiffness = Spring.StiffnessMediumLow))
    }

    private companion object {
        const val SKIP_AT = 0.25f
        const val FLY_MS = 150
        const val WAIT_MS = 600L
    }
}

/** [track] is what shows: when it changes after a swipe, the new one slides in. */
@Composable
fun rememberSkipSwipe(track: Any?, onNext: () -> Unit, onPrevious: () -> Unit): SkipSwipe {
    val scope = rememberCoroutineScope()
    val next by rememberUpdatedState(onNext)
    val previous by rememberUpdatedState(onPrevious)
    val swipe = remember { SkipSwipe(scope, { next() }, { previous() }) }
    LaunchedEffect(track) { swipe.arrived() }
    return swipe
}

/** Where the finger swipes sideways to skip. */
fun Modifier.skipGesture(swipe: SkipSwipe): Modifier =
    pointerInput(swipe) {
        detectHorizontalDragGestures(
            onDragStart = { swipe.start(size.width.toFloat()) },
            onDragEnd = swipe::release,
            onDragCancel = swipe::cancel,
            onHorizontalDrag = { change, dx ->
                change.consume()
                swipe.drag(dx)
            },
        )
    }

/** What moves with the swipe, fading as it leaves. */
fun Modifier.skipOffset(swipe: SkipSwipe): Modifier =
    graphicsLayer {
        val x = swipe.offset.value
        translationX = x
        alpha = 1f - (abs(x) / size.width.coerceAtLeast(1f)).coerceIn(0f, 0.6f)
    }
