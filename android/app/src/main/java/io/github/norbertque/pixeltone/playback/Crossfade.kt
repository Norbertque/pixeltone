package io.github.norbertque.pixeltone.playback

import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

/** How far ahead of a crossfade the next track is made ready, paused, on the other player. */
const val CROSSFADE_PREPARE_MS = 4_000L

/** What the playing track is about to do, as far as crossfading goes. */
sealed interface CrossfadeStep {
    /** Nothing: no crossfade, or not yet. */
    data object None : CrossfadeStep

    /** Get the next track ready on the other player. */
    data object Prepare : CrossfadeStep

    /** Start the next track now and blend the two over [fadeMs]. */
    data class Start(val fadeMs: Long) : CrossfadeStep
}

/**
 * Whether the track that plays should fade into the next one, by Windows' rules: only when it
 * plays on to a next track by itself (not when it repeats, nor when the sleep timer stops it at
 * its end), only between albums unless [Sound.crossfadeAlbums], never longer than half the track.
 */
fun crossfadeStep(
    sound: Sound,
    playing: Boolean,
    positionMs: Long,
    durationMs: Long,
    hasNext: Boolean,
    repeatsOne: Boolean,
    stopsAtEnd: Boolean,
    currentAlbum: String?,
    nextAlbum: String?,
): CrossfadeStep {
    if (sound.crossfadeMs <= 0 || !playing || !hasNext || repeatsOne || stopsAtEnd || durationMs <= 0) return CrossfadeStep.None
    if (!sound.crossfadeAlbums && sameAlbum(currentAlbum, nextAlbum)) return CrossfadeStep.None
    val length = minOf(sound.crossfadeMs.toLong(), durationMs / 2)
    val remaining = durationMs - positionMs
    return when {
        remaining <= 0 -> CrossfadeStep.None
        remaining <= length -> CrossfadeStep.Start(remaining)
        remaining <= length + CROSSFADE_PREPARE_MS -> CrossfadeStep.Prepare
        else -> CrossfadeStep.None
    }
}

/** Consecutive tracks of one album, kept gapless unless the user wants them crossfaded (Windows' `same_album`). */
fun sameAlbum(a: String?, b: String?): Boolean = !a.isNullOrEmpty() && a == b

/** The two volumes [fraction] of the way through an equal-power crossfade: the outgoing track's, the incoming one's. */
fun crossfadeVolumes(fraction: Float): Pair<Float, Float> {
    val t = fraction.coerceIn(0f, 1f) * PI.toFloat() / 2
    return cos(t) to sin(t)
}
