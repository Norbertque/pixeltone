package io.github.norbertque.pixeltone.ui.player

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ClearAll
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.DragIndicator
import androidx.compose.material.icons.filled.GraphicEq
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.Shuffle
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.IconToggleButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.playback.PlaybackState
import io.github.norbertque.pixeltone.playback.PlayerControls
import io.github.norbertque.pixeltone.playback.QueueEntry
import io.github.norbertque.pixeltone.ui.Message
import io.github.norbertque.pixeltone.ui.library.Cover

/** How many tracks, then shuffle and Clear, as on Windows. */
@Composable
fun QueueHeader(state: PlaybackState, controls: PlayerControls, editable: Boolean, modifier: Modifier = Modifier) {
    Row(modifier.fillMaxWidth().height(48.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(
            pluralStringResource(R.plurals.tracks_count, state.queue.size, state.queue.size),
            Modifier.weight(1f),
            style = MaterialTheme.typography.titleSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        if (editable) {
            IconToggleButton(
                checked = state.shuffle,
                onCheckedChange = { controls.toggleShuffle() },
                colors = IconButtonDefaults.iconToggleButtonColors(
                    checkedContainerColor = MaterialTheme.colorScheme.secondaryContainer,
                    checkedContentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                ),
            ) {
                Icon(Icons.Default.Shuffle, stringResource(R.string.shuffle))
            }
            TextButton(onClick = controls::clearQueue, enabled = state.queue.size > 1) {
                Icon(Icons.Default.ClearAll, null, Modifier.size(18.dp))
                Spacer(Modifier.width(8.dp))
                Text(stringResource(R.string.clear))
            }
        }
    }
}

/**
 * The queue's rows: the one that plays stands out, those played are dimmed. A row dragged by its
 * handle moves in this list, and the player gets one move when it's dropped.
 */
@Composable
fun QueueList(
    state: PlaybackState,
    controls: PlayerControls,
    editable: Boolean,
    modifier: Modifier = Modifier,
    contentPadding: PaddingValues = PaddingValues(start = 8.dp, end = 8.dp, top = 8.dp, bottom = 24.dp),
) {
    if (state.queue.isEmpty()) {
        Message(stringResource(R.string.queue_empty), modifier)
        return
    }
    val list = rememberLazyListState(initialFirstVisibleItemIndex = (state.index - 1).coerceAtLeast(0))
    // Each row is its position in the player's queue, in the order shown.
    var rows by remember(state.queue) { mutableStateOf(state.queue.indices.toList()) }
    var dragged by remember { mutableStateOf<Int?>(null) }
    var offset by remember { mutableFloatStateOf(0f) }
    LazyColumn(state = list, modifier = modifier.fillMaxSize(), contentPadding = contentPadding, verticalArrangement = Arrangement.spacedBy(2.dp)) {
        itemsIndexed(rows, key = { _, row -> row }) { _, row ->
            val isDragged = row == dragged
            QueueRow(
                entry = state.queue[row],
                current = row == state.index,
                playing = row == state.index && state.isPlaying,
                played = state.index >= 0 && row < state.index,
                editable = editable,
                modifier = Modifier
                    .zIndex(if (isDragged) 1f else 0f)
                    .graphicsLayer { translationY = if (isDragged) offset else 0f },
                handle = Modifier.pointerInput(row) {
                    detectDragGestures(
                        onDragStart = {
                            dragged = row
                            offset = 0f
                        },
                        onDrag = { change, amount ->
                            change.consume()
                            offset += amount.y
                            val shown = list.layoutInfo.visibleItemsInfo
                            val me = shown.firstOrNull { it.key == row } ?: return@detectDragGestures
                            val middle = (me.offset + me.size / 2 + offset).toInt()
                            val over = shown.firstOrNull { it.key != row && middle in it.offset until it.offset + it.size }
                                ?: return@detectDragGestures
                            val from = rows.indexOf(row)
                            val to = rows.indexOf(over.key as Int)
                            rows = rows.toMutableList().apply { add(to, removeAt(from)) }
                            // The row now sits where the other one was: keep it under the finger.
                            offset += me.offset - over.offset
                        },
                        onDragEnd = {
                            val to = rows.indexOf(row)
                            dragged = null
                            offset = 0f
                            if (to != row) controls.move(row, to)
                        },
                        onDragCancel = {
                            dragged = null
                            offset = 0f
                            rows = state.queue.indices.toList()
                        },
                    )
                },
                onClick = { controls.jumpTo(row) },
                onRemove = { controls.remove(row) },
            )
        }
    }
}

@Composable
private fun QueueRow(
    entry: QueueEntry,
    current: Boolean,
    playing: Boolean,
    played: Boolean,
    editable: Boolean,
    modifier: Modifier,
    handle: Modifier,
    onClick: () -> Unit,
    onRemove: () -> Unit,
) {
    val background = if (current) MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.surface.copy(alpha = 0f)
    Row(
        modifier
            .fillMaxWidth()
            .height(68.dp)
            .clip(MaterialTheme.shapes.large)
            .background(background)
            .clickable(onClick = onClick)
            .padding(start = if (editable) 0.dp else 12.dp, end = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (editable) {
            Icon(
                Icons.Default.DragIndicator,
                stringResource(R.string.move),
                handle.padding(horizontal = 8.dp).size(24.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Row(Modifier.weight(1f).alpha(if (played) 0.55f else 1f), verticalAlignment = Alignment.CenterVertically) {
            Cover(entry.cover, Modifier.size(48.dp), MaterialTheme.shapes.medium)
            Column(Modifier.weight(1f).padding(horizontal = 12.dp)) {
                Text(
                    entry.title,
                    style = MaterialTheme.typography.titleSmall,
                    color = if (current) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    entry.artist ?: stringResource(R.string.unknown_artist),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
        when {
            current -> Icon(
                if (playing) Icons.Default.GraphicEq else Icons.Default.Pause,
                stringResource(R.string.now_playing),
                Modifier.padding(horizontal = 12.dp),
                tint = MaterialTheme.colorScheme.primary,
            )
            editable -> IconButton(onClick = onRemove) {
                Icon(Icons.Default.Close, stringResource(R.string.remove_from_queue), tint = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}
