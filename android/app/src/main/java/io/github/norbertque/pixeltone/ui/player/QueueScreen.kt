package io.github.norbertque.pixeltone.ui.player

import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.DragHandle
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.zIndex
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.playback.PlaybackState
import io.github.norbertque.pixeltone.playback.PlayerConnection
import io.github.norbertque.pixeltone.playback.PlayerControls
import io.github.norbertque.pixeltone.playback.QueueEntry
import io.github.norbertque.pixeltone.ui.BackButton
import io.github.norbertque.pixeltone.ui.Message

@Composable
fun QueueScreen(player: PlayerConnection, onBack: () -> Unit) {
    val state by player.state.collectAsStateWithLifecycle()
    QueueContent(state, player, onBack)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QueueContent(state: PlaybackState, controls: PlayerControls, onBack: () -> Unit) {
    Scaffold(
        topBar = { TopAppBar(title = { Text(stringResource(R.string.queue)) }, navigationIcon = { BackButton(onBack) }) },
    ) { padding ->
        if (state.queue.isEmpty()) {
            Message(stringResource(R.string.queue_empty), Modifier.padding(padding))
        } else {
            Rows(state, controls, padding)
        }
    }
}

/** The queue; a row dragged by its handle moves in this list, and the player gets one move when it's dropped. */
@Composable
private fun Rows(state: PlaybackState, controls: PlayerControls, padding: PaddingValues) {
    val list = rememberLazyListState(initialFirstVisibleItemIndex = state.index.coerceAtLeast(0))
    // Each row is its position in the player's queue, in the order shown.
    var rows by remember(state.queue) { mutableStateOf(state.queue.indices.toList()) }
    var dragged by remember { mutableStateOf<Int?>(null) }
    var offset by remember { mutableFloatStateOf(0f) }
    LazyColumn(state = list, modifier = Modifier.fillMaxSize(), contentPadding = padding) {
        itemsIndexed(rows, key = { _, row -> row }) { _, row ->
            val isDragged = row == dragged
            QueueRow(
                entry = state.queue[row],
                playing = row == state.index,
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
    playing: Boolean,
    modifier: Modifier,
    handle: Modifier,
    onClick: () -> Unit,
    onRemove: () -> Unit,
) {
    ListItem(
        headlineContent = {
            Text(
                entry.title,
                color = if (playing) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        },
        supportingContent = {
            Text(entry.artist ?: stringResource(R.string.unknown_artist), maxLines = 1, overflow = TextOverflow.Ellipsis)
        },
        leadingContent = { Icon(Icons.Default.DragHandle, stringResource(R.string.move), handle) },
        trailingContent = {
            IconButton(onClick = onRemove) {
                Icon(Icons.Default.Close, stringResource(R.string.remove_from_queue))
            }
        },
        colors = if (playing) {
            ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.secondaryContainer)
        } else {
            ListItemDefaults.colors()
        },
        modifier = modifier.clickable(onClick = onClick),
    )
}
