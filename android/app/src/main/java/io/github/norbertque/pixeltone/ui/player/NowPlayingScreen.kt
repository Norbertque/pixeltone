package io.github.norbertque.pixeltone.ui.player

import androidx.activity.compose.BackHandler
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.PlaylistAdd
import androidx.compose.material.icons.automirrored.filled.QueueMusic
import androidx.compose.material.icons.filled.Bedtime
import androidx.compose.material.icons.filled.Computer
import androidx.compose.material.icons.filled.Devices
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material.icons.filled.FavoriteBorder
import androidx.compose.material.icons.filled.Groups
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.Lyrics
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Repeat
import androidx.compose.material.icons.filled.RepeatOne
import androidx.compose.material.icons.filled.Shuffle
import androidx.compose.material.icons.filled.SkipNext
import androidx.compose.material.icons.filled.SkipPrevious
import androidx.compose.material.icons.filled.Usb
import androidx.compose.material3.BottomSheetScaffold
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.IconToggleButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberBottomSheetScaffoldState
import androidx.compose.material3.rememberStandardBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.media3.common.Player
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.playback.PlaybackState
import io.github.norbertque.pixeltone.playback.PlayerConnection
import io.github.norbertque.pixeltone.playback.PlayerControls
import io.github.norbertque.pixeltone.playback.Sleep
import io.github.norbertque.pixeltone.playback.nextRepeatMode
import io.github.norbertque.pixeltone.ui.Message
import io.github.norbertque.pixeltone.ui.WavySeekBar
import io.github.norbertque.pixeltone.ui.formatDuration
import io.github.norbertque.pixeltone.ui.library.Cover
import io.github.norbertque.pixeltone.ui.theme.Emphasized
import io.github.norbertque.pixeltone.ui.theme.ShapeXlPlus
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@Composable
fun NowPlayingScreen(
    player: PlayerConnection,
    onBack: () -> Unit,
    playingOn: String?,
    outputBadge: String?,
    onPlayOn: () -> Unit,
    onJam: (() -> Unit)? = null,
    favourite: Boolean? = null,
    onFavourite: () -> Unit = {},
    onAddToPlaylist: (() -> Unit)? = null,
    sleep: Sleep = Sleep.Off,
    onSleep: () -> Unit = {},
    queueEditable: Boolean = true,
    lyrics: @Composable () -> Unit = {},
    firstPanel: NowPlayingPanel = NowPlayingPanel.Lyrics,
) {
    val state by player.state.collectAsStateWithLifecycle()
    val position by player.positionMs.collectAsStateWithLifecycle(initialValue = 0L)
    NowPlayingContent(
        state, position, player, onBack, playingOn, outputBadge, onPlayOn, onJam,
        favourite, onFavourite, onAddToPlaylist, sleepText(sleep), onSleep, queueEditable, lyrics, firstPanel = firstPanel,
    )
}

/** What the panel pulled up from under Now Playing shows. */
enum class NowPlayingPanel { Lyrics, Queue }

/** The panel's tabs, above the navigation bar while it's down. */
private val PANEL_PEEK = 72.dp

/** How far a swipe down closes Now Playing, and a swipe up pulls up the panel. */
private val CLOSE_AT = 120.dp
private val OPEN_AT = 64.dp

/**
 * The track that plays, big. A swipe down closes it, a swipe sideways on the cover skips, and a
 * swipe up (or a tab at the bottom) pulls up the panel with the lyrics and the queue.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NowPlayingContent(
    state: PlaybackState,
    positionMs: Long,
    controls: PlayerControls,
    onBack: () -> Unit,
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
    /** What the sleep timer will do ("Stops in 23:41"); null while it's off. */
    sleep: String? = null,
    onSleep: () -> Unit = {},
    /** The queue can be changed here (a PC's can't). */
    queueEditable: Boolean = true,
    /** The lyrics of what plays, for the panel. */
    lyrics: @Composable () -> Unit = {},
    /** The panel that's up from the start (a screenshot's); null: it waits at the bottom. */
    opened: NowPlayingPanel? = null,
    /** What the panel shows when it's first pulled up. */
    firstPanel: NowPlayingPanel = NowPlayingPanel.Lyrics,
) {
    val scope = rememberCoroutineScope()
    val sheet = rememberStandardBottomSheetState(
        initialValue = if (opened != null) SheetValue.Expanded else SheetValue.PartiallyExpanded,
        skipHiddenState = true,
    )
    var panel by rememberSaveable { mutableStateOf(opened ?: firstPanel) }
    val raised = sheet.currentValue == SheetValue.Expanded || sheet.targetValue == SheetValue.Expanded
    val open: (NowPlayingPanel) -> Unit = remember(scope, sheet) {
        { which ->
            panel = which
            scope.launch { sheet.expand() }
        }
    }
    BackHandler(enabled = raised) { scope.launch { sheet.partialExpand() } }
    val navigationBar = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
    BottomSheetScaffold(
        sheetContent = {
            Panel(
                panel,
                raised,
                onPick = { which -> if (raised && which == panel) scope.launch { sheet.partialExpand() } else open(which) },
                Modifier.padding(bottom = navigationBar),
            ) { shown ->
                when (shown) {
                    NowPlayingPanel.Lyrics -> lyrics()
                    NowPlayingPanel.Queue -> Column {
                        QueueHeader(state, controls, queueEditable, Modifier.padding(start = 24.dp, end = 12.dp))
                        QueueList(state, controls, queueEditable, Modifier.weight(1f))
                    }
                }
            }
        },
        scaffoldState = rememberBottomSheetScaffoldState(sheet),
        sheetPeekHeight = PANEL_PEEK + navigationBar,
        sheetContainerColor = MaterialTheme.colorScheme.surfaceContainer,
        sheetDragHandle = null,
        containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
    ) { padding ->
        val glow = MaterialTheme.colorScheme.primaryContainer
        val back by rememberUpdatedState(onBack)
        // A swipe down closes Now Playing (what shows follows it a little), a swipe up pulls up the panel.
        var pull by remember { mutableFloatStateOf(0f) }
        var pulling by remember { mutableStateOf(false) }
        val shownPull by animateFloatAsState(if (pulling) pull.coerceAtLeast(0f) else 0f, label = "pull")
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
                }
                .pointerInput(Unit) {
                    detectVerticalDragGestures(
                        onDragStart = {
                            pull = 0f
                            pulling = true
                        },
                        onDragEnd = {
                            when {
                                // It stays where the finger left it while it goes.
                                pull > CLOSE_AT.toPx() -> back()
                                pull < -OPEN_AT.toPx() -> {
                                    pulling = false
                                    open(panel)
                                }
                                else -> pulling = false
                            }
                        },
                        onDragCancel = { pulling = false },
                        onVerticalDrag = { change, dy ->
                            change.consume()
                            pull += dy
                        },
                    )
                },
        ) {
            Column(
                Modifier
                    .fillMaxSize()
                    .graphicsLayer { translationY = shownPull * 0.4f }
                    .statusBarsPadding()
                    .padding(padding),
            ) {
                val entry = state.current
                Header(entry?.album, onBack)
                if (entry == null) {
                    Message(stringResource(R.string.queue_empty))
                    return@Column
                }
                val skip = rememberSkipSwipe(entry, controls::next, controls::previous)
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
                            .skipGesture(skip)
                            .skipOffset(skip)
                            .shadow(24.dp, ShapeXlPlus, ambientColor = glow, spotColor = Color.Black.copy(alpha = 0.3f)),
                        ShapeXlPlus,
                    )
                    Column(Modifier.fillMaxWidth().padding(top = 16.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            // As wide as the heart: the title sits in the middle of the screen.
                            if (favourite != null) Spacer(Modifier.width(48.dp))
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

/** Close on the left; where the music comes from, in the middle of the screen. */
@Composable
private fun Header(album: String?, onBack: () -> Unit) {
    Box(Modifier.fillMaxWidth().padding(4.dp), contentAlignment = Alignment.Center) {
        IconButton(onClick = onBack, Modifier.align(Alignment.CenterStart)) {
            Icon(Icons.Default.KeyboardArrowDown, stringResource(R.string.close_player))
        }
        // As far from both edges as the button is wide, so it's centred on the screen.
        Column(Modifier.padding(horizontal = 52.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            Text(
                stringResource(R.string.playing_from).uppercase(),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                album ?: stringResource(R.string.now_playing),
                style = MaterialTheme.typography.titleSmall,
                textAlign = TextAlign.Center,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

/**
 * The panel under Now Playing: a handle and its tabs while it's down; pulled up, the lyrics or the
 * queue. What it shows is only there while it's up.
 */
@Composable
private fun Panel(
    panel: NowPlayingPanel,
    raised: Boolean,
    onPick: (NowPlayingPanel) -> Unit,
    modifier: Modifier,
    content: @Composable (NowPlayingPanel) -> Unit,
) {
    Column(modifier.fillMaxWidth().fillMaxHeight(0.9f)) {
        Box(
            Modifier
                .padding(top = 10.dp, bottom = 6.dp)
                .size(width = 32.dp, height = 4.dp)
                .clip(CircleShape)
                .background(MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f))
                .align(Alignment.CenterHorizontally),
        )
        Row(
            Modifier.fillMaxWidth().height(48.dp).padding(horizontal = 16.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            NowPlayingPanel.entries.forEach { which ->
                val selected = raised && which == panel
                TextButton(
                    onClick = { onPick(which) },
                    modifier = Modifier.weight(1f).fillMaxHeight(),
                    colors = ButtonDefaults.textButtonColors(
                        containerColor = if (selected) MaterialTheme.colorScheme.secondaryContainer else Color.Transparent,
                        contentColor = if (selected) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.colorScheme.onSurfaceVariant,
                    ),
                ) {
                    Icon(if (which == NowPlayingPanel.Lyrics) Icons.Default.Lyrics else Icons.AutoMirrored.Filled.QueueMusic, null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text(stringResource(if (which == NowPlayingPanel.Lyrics) R.string.lyrics else R.string.queue))
                }
            }
        }
        if (raised) {
            Box(
                Modifier
                    .weight(1f)
                    // What scrolls fades out under the tabs.
                    .graphicsLayer(compositingStrategy = CompositingStrategy.Offscreen)
                    .drawWithContent {
                        drawContent()
                        drawRect(Brush.verticalGradient(listOf(Color.Transparent, Color.Black), endY = 16.dp.toPx()), blendMode = BlendMode.DstIn)
                    },
            ) {
                content(panel)
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
