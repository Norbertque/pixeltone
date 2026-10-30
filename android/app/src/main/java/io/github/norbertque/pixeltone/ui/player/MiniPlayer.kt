package io.github.norbertque.pixeltone.ui.player

import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.SkipNext
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.playback.QueueEntry
import io.github.norbertque.pixeltone.ui.library.Cover

/** How far a swipe up on the mini player opens Now Playing. */
private val OPEN_AT = 24.dp

/**
 * What plays, on a card above the navigation. A tap or a swipe up opens Now Playing; a swipe
 * sideways skips to the next track or back to the previous one.
 */
@Composable
fun MiniPlayer(
    entry: QueueEntry,
    isPlaying: Boolean,
    onToggle: () -> Unit,
    onOpen: () -> Unit,
    playingOn: String? = null,
    onNext: () -> Unit = {},
    onPrevious: () -> Unit = {},
) {
    val skip = rememberSkipSwipe(entry, onNext, onPrevious)
    val open by rememberUpdatedState(onOpen)
    Surface(
        onClick = onOpen,
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 12.dp, vertical = 6.dp)
            .pointerInput(Unit) {
                var pulled = 0f
                detectVerticalDragGestures(
                    onDragStart = { pulled = 0f },
                    onDragEnd = { if (pulled < -OPEN_AT.toPx()) open() },
                    onVerticalDrag = { change, dy ->
                        change.consume()
                        pulled += dy
                    },
                )
            }
            .skipGesture(skip),
        shape = MaterialTheme.shapes.extraLarge,
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
        shadowElevation = 6.dp,
    ) {
        Row(Modifier.padding(8.dp), verticalAlignment = Alignment.CenterVertically) {
            // The track slides within its own room, under neither button.
            Row(Modifier.weight(1f).clipToBounds()) {
                Row(Modifier.skipOffset(skip), verticalAlignment = Alignment.CenterVertically) {
                    Cover(entry.cover, Modifier.size(52.dp), MaterialTheme.shapes.large)
                    Column(Modifier.weight(1f).padding(horizontal = 12.dp)) {
                        Text(entry.title, style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        Text(
                            listOfNotNull(playingOn, entry.artist ?: stringResource(R.string.unknown_artist)).joinToString(" · "),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
            }
            FilledIconButton(onClick = onToggle, modifier = Modifier.size(48.dp)) {
                if (isPlaying) {
                    Icon(Icons.Default.Pause, stringResource(R.string.pause))
                } else {
                    Icon(Icons.Default.PlayArrow, stringResource(R.string.play))
                }
            }
            IconButton(onClick = onNext) { Icon(Icons.Default.SkipNext, stringResource(R.string.next)) }
        }
    }
}
