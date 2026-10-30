package io.github.norbertque.pixeltone.ui.player

import androidx.compose.animation.Crossfade
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.PlaylistAdd
import androidx.compose.material.icons.automirrored.filled.QueueMusic
import androidx.compose.material.icons.filled.Computer
import androidx.compose.material.icons.filled.Devices
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material.icons.filled.FavoriteBorder
import androidx.compose.material.icons.filled.Bedtime
import androidx.compose.material.icons.filled.Groups
import androidx.compose.material.icons.filled.Lyrics
import androidx.compose.material.icons.filled.Usb
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Repeat
import androidx.compose.material.icons.filled.RepeatOne
import androidx.compose.material.icons.filled.Shuffle
import androidx.compose.material.icons.filled.SkipNext
import androidx.compose.material.icons.filled.SkipPrevious
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.IconToggleButton
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import io.github.norbertque.pixeltone.ui.theme.ShapeXlPlus
import io.github.norbertque.pixeltone.ui.theme.Emphasized
import io.github.norbertque.pixeltone.ui.WavySeekBar
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.draw.drawBehind
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.media3.common.Player
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.playback.PlaybackState
import io.github.norbertque.pixeltone.playback.Sleep
import io.github.norbertque.pixeltone.playback.PlayerConnection
import io.github.norbertque.pixeltone.playback.PlayerControls
import io.github.norbertque.pixeltone.playback.nextRepeatMode
import io.github.norbertque.pixeltone.ui.Message
import io.github.norbertque.pixeltone.ui.formatDuration
import io.github.norbertque.pixeltone.ui.library.Cover
import kotlinx.coroutines.delay

@Composable
fun NowPlayingScreen(
    player: PlayerConnection,
    onBack: () -> Unit,
    onQueue: () -> Unit,
    playingOn: String?,
    outputBadge: String?,
    onPlayOn: () -> Unit,
    onJam: (() -> Unit)? = null,
    favourite: Boolean? = null,
    onFavourite: () -> Unit = {},
    onAddToPlaylist: (() -> Unit)? = null,
    onLyrics: () -> Unit = {},
    sleep: Sleep = Sleep.Off,
    onSleep: () -> Unit = {},
) {
    val state by player.state.collectAsStateWithLifecycle()
    val position by player.positionMs.collectAsStateWithLifecycle(initialValue = 0L)
    NowPlayingContent(
        state, position, player, onBack, onQueue, playingOn, outputBadge, onPlayOn, onJam,
        favourite, onFavourite, onAddToPlaylist, onLyrics, sleepText(sleep), onSleep,
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NowPlayingContent(
    state: PlaybackState,
    positionMs: Long,
    controls: PlayerControls,
    onBack: () -> Unit,
    onQueue: () -> Unit,
    /** The PC playback is on, or null on the phone. */
    playingOn: String? = null,
    /** The PC's output, e.g. "Exclusive • 24-bit/96 kHz • Bit-perfect". */
    outputBadge: String? = null,
    onPlayOn: () -> Unit = {},
    /** The jam on the PC playback is on; null on the phone. */
    onJam: (() -> Unit)? = null,
    /** Whether the phone's track that plays is a favourite; null hides the heart (a PC's track). */
    favourite: Boolean? = null,
    onFavourite: () -> Unit = {},
    onAddToPlaylist: (() -> Unit)? = null,
    onLyrics: () -> Unit = {},
    /** What the sleep timer will do ("Stops in 23:41"); null while it's off. */
    sleep: String? = null,
    onSleep: () -> Unit = {},
) {
    val glow = MaterialTheme.colorScheme.primaryContainer
    Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.surfaceContainerLow) {
    Box(
        Modifier
            .fillMaxSize()
            // The cover's colour glows in from the top, as on Windows.
            .drawBehind {
                drawRect(
                    Brush.radialGradient(
                        listOf(glow.copy(alpha = 0.75f), Color.Transparent),
                        center = Offset(size.width * 0.15f, 0f),
                        radius = size.maxDimension * 0.8f,
                    ),
                )
            },
    ) {
        Column(Modifier.fillMaxSize().systemBarsPadding()) {
            val entry = state.current
            Row(Modifier.fillMaxWidth().padding(horizontal = 4.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                IconButton(onClick = onBack) { Icon(Icons.Default.KeyboardArrowDown, stringResource(R.string.close_player)) }
                Column(Modifier.weight(1f), horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(
                        stringResource(R.string.playing_from).uppercase(),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        entry?.album ?: stringResource(R.string.now_playing),
                        style = MaterialTheme.typography.titleSmall,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                IconButton(onClick = onLyrics) { Icon(Icons.Default.Lyrics, stringResource(R.string.lyrics)) }
                IconButton(onClick = onQueue) { Icon(Icons.AutoMirrored.Filled.QueueMusic, stringResource(R.string.queue)) }
            }
            if (entry == null) {
                Message(stringResource(R.string.queue_empty))
                return@Column
            }
            Column(
                Modifier.fillMaxSize().padding(horizontal = 24.dp, vertical = 8.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.SpaceEvenly,
            ) {
                Cover(
                    entry.cover,
                    Modifier
                        .weight(1f, fill = false)
                        .aspectRatio(1f, matchHeightConstraintsFirst = true)
                        .shadow(24.dp, ShapeXlPlus, ambientColor = glow, spotColor = Color.Black.copy(alpha = 0.3f)),
                    ShapeXlPlus,
                )
                Column(Modifier.fillMaxWidth().padding(top = 16.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            entry.title,
                            Modifier.weight(1f, fill = false),
                            style = MaterialTheme.typography.headlineMedium,
                            fontWeight = Emphasized,
                            textAlign = TextAlign.Center,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                        if (favourite != null) {
                            IconToggleButton(checked = favourite, onCheckedChange = { onFavourite() }) {
                                if (favourite) {
                                    Icon(Icons.Default.Favorite, stringResource(R.string.unfavourite), tint = MaterialTheme.colorScheme.primary)
                                } else {
                                    Icon(Icons.Default.FavoriteBorder, stringResource(R.string.favourite))
                                }
                            }
                        }
                    }
                    Text(
                        entry.artist ?: stringResource(R.string.unknown_artist),
                        style = MaterialTheme.typography.titleLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        textAlign = TextAlign.Center,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    when {
                        playingOn != null -> PlayingOn(playingOn, outputBadge)
                        outputBadge != null -> Output(outputBadge)
                    }
                }
                SeekBar(positionMs, state.durationMs, state.isPlaying, controls::seekTo)
                // What a toggle now does, shown for a moment under the controls.
                var hint by remember { mutableStateOf<Hint?>(null) }
                LaunchedEffect(hint) {
                    if (hint != null) {
                        delay(2_500)
                        hint = null
                    }
                }
                Controls(state, controls, onHint = { hint = Hint(it, (hint?.id ?: 0) + 1) })
                HintLine(hint)
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                    Extra(Icons.Default.Devices, stringResource(R.string.play_on), on = playingOn != null, onClick = onPlayOn)
                    if (onJam != null) Extra(Icons.Default.Groups, stringResource(R.string.jam), on = false, onClick = onJam)
                    if (onAddToPlaylist != null) {
                        Extra(Icons.AutoMirrored.Filled.PlaylistAdd, stringResource(R.string.add_to_playlist), on = false, onClick = onAddToPlaylist)
                    }
                    Extra(Icons.Default.Bedtime, stringResource(R.string.sleep_timer), on = sleep != null, onClick = onSleep)
                    if (sleep != null) {
                        Text(
                            sleep,
                            Modifier.clickable(onClick = onSleep),
                            style = MaterialTheme.typography.labelLarge,
                            color = MaterialTheme.colorScheme.primary,
                        )
                    }
                }
            }
        }
    }
    }
}

/** A secondary action under the controls: a tonal circle, filled while it's on. */
@Composable
private fun Extra(icon: ImageVector, label: String, on: Boolean, onClick: () -> Unit) {
    FilledTonalIconButton(
        onClick = onClick,
        colors = if (on) {
            IconButtonDefaults.filledTonalIconButtonColors(
                containerColor = MaterialTheme.colorScheme.primaryContainer,
                contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
            )
        } else {
            IconButtonDefaults.filledTonalIconButtonColors(
                containerColor = MaterialTheme.colorScheme.surfaceContainerHighest,
                contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        },
    ) {
        Icon(icon, label)
    }
}

@Composable
private fun SeekBar(positionMs: Long, durationMs: Long, playing: Boolean, onSeek: (Long) -> Unit) {
    val fraction = if (durationMs > 0) (positionMs.toFloat() / durationMs).coerceIn(0f, 1f) else 0f
    Column(Modifier.fillMaxWidth()) {
        WavySeekBar(
            value = fraction,
            playing = playing,
            onSeek = { if (durationMs > 0) onSeek((it * durationMs).toLong()) },
            label = stringResource(R.string.seek),
        )
        Row(Modifier.fillMaxWidth().padding(horizontal = 4.dp)) {
            Text(formatDuration(positionMs.coerceAtMost(durationMs)), style = MaterialTheme.typography.labelLarge)
            Spacer(Modifier.weight(1f))
            Text(formatDuration(durationMs), style = MaterialTheme.typography.labelLarge)
        }
    }
}

@Composable
private fun Controls(state: PlaybackState, controls: PlayerControls, onHint: (String) -> Unit) {
    val shuffleHints = mapOf(true to stringResource(R.string.shuffle_on_hint), false to stringResource(R.string.shuffle_off_hint))
    val repeatHints = mapOf(
        Player.REPEAT_MODE_OFF to stringResource(R.string.repeat_off_hint),
        Player.REPEAT_MODE_ALL to stringResource(R.string.repeat_all_hint),
        Player.REPEAT_MODE_ONE to stringResource(R.string.repeat_one_hint),
    )
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceEvenly,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Toggle(Icons.Default.Shuffle, stringResource(R.string.shuffle), on = state.shuffle) {
            controls.toggleShuffle()
            onHint(shuffleHints.getValue(!state.shuffle))
        }
        FilledTonalIconButton(onClick = controls::previous, Modifier.size(width = 72.dp, height = 56.dp), shape = CircleShape) {
            Icon(Icons.Default.SkipPrevious, stringResource(R.string.previous), Modifier.size(28.dp))
        }
        FilledIconButton(onClick = controls::toggle, Modifier.size(96.dp), shape = CircleShape) {
            if (state.isPlaying) {
                Icon(Icons.Default.Pause, stringResource(R.string.pause), Modifier.size(44.dp))
            } else {
                Icon(Icons.Default.PlayArrow, stringResource(R.string.play), Modifier.size(44.dp))
            }
        }
        FilledTonalIconButton(onClick = controls::next, Modifier.size(width = 72.dp, height = 56.dp), shape = CircleShape) {
            Icon(Icons.Default.SkipNext, stringResource(R.string.next), Modifier.size(28.dp))
        }
        val (icon, label) = when (state.repeatMode) {
            Player.REPEAT_MODE_ONE -> Icons.Default.RepeatOne to R.string.repeat_one
            Player.REPEAT_MODE_ALL -> Icons.Default.Repeat to R.string.repeat_all
            else -> Icons.Default.Repeat to R.string.repeat_off
        }
        Toggle(icon, stringResource(label), on = state.repeatMode != Player.REPEAT_MODE_OFF) {
            controls.cycleRepeat()
            onHint(repeatHints.getValue(nextRepeatMode(state.repeatMode)))
        }
    }
}

/** Shuffle and repeat: a tinted circle behind the icon while they're on. */
@Composable
private fun Toggle(icon: ImageVector, label: String, on: Boolean, onClick: () -> Unit) {
    IconToggleButton(
        checked = on,
        onCheckedChange = { onClick() },
        colors = IconButtonDefaults.iconToggleButtonColors(
            contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
            checkedContainerColor = MaterialTheme.colorScheme.secondaryContainer,
            checkedContentColor = MaterialTheme.colorScheme.onSecondaryContainer,
        ),
    ) {
        Icon(icon, label)
    }
}

@Composable
private fun PlayingOn(pc: String, badge: String?) {
    Row(Modifier.padding(top = 12.dp), verticalAlignment = Alignment.CenterVertically) {
        Icon(Icons.Default.Computer, null, Modifier.size(18.dp), tint = MaterialTheme.colorScheme.primary)
        Spacer(Modifier.width(8.dp))
        Column {
            Text(stringResource(R.string.playing_on, pc), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
            if (!badge.isNullOrEmpty()) {
                Text(badge, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}

/** Where the phone's own playback goes, when it isn't Android's output: a USB DAC. */
@Composable
private fun Output(badge: String) {
    Row(Modifier.padding(top = 12.dp), verticalAlignment = Alignment.CenterVertically) {
        Icon(Icons.Default.Usb, null, Modifier.size(18.dp), tint = MaterialTheme.colorScheme.primary)
        Spacer(Modifier.width(8.dp))
        Text(badge, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
    }
}

private data class Hint(val text: String, val id: Int)

@Composable
private fun HintLine(hint: Hint?) {
    // The line keeps its room without a hint, so the controls never jump.
    Box(Modifier.fillMaxWidth().height(40.dp), contentAlignment = Alignment.Center) {
        Crossfade(targetState = hint, label = "hint") { shown ->
            if (shown != null) {
                Text(
                    shown.text,
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                    maxLines = 2,
                )
            }
        }
    }
}
