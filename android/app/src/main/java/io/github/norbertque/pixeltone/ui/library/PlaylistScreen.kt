package io.github.norbertque.pixeltone.ui.library

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.DragHandle
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Shuffle
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.ListedTrack
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.ui.BackButton
import io.github.norbertque.pixeltone.ui.Message
import io.github.norbertque.pixeltone.ui.formatDuration

/** A row of a list: what was listed, and the track on the phone now (null when it's gone). */
data class ListRow(val listed: ListedTrack, val track: Track?)

/** Favourites ([id] null) or a playlist. */
@Composable
fun PlaylistScreen(app: PixeltoneApp, id: Long?, onBack: () -> Unit) {
    val lists by app.lists.lists.collectAsStateWithLifecycle()
    val index by app.library.index.collectAsStateWithLifecycle()
    val playlist = id?.let { lists.playlist(it) }
    if (id != null && playlist == null) {
        Message(stringResource(R.string.playlist_gone))
        return
    }
    val listed = playlist?.tracks ?: lists.favourites
    val rows = listed.map { ListRow(it, index?.trackByFile(it.file)) }
    var renaming by remember { mutableStateOf(false) }
    val player = app.player
    fun play(start: Int) {
        val available = rows.mapNotNull(ListRow::track)
        if (available.isEmpty()) return
        val before = rows.take(start).count { it.track != null }
        player.play(available, before.coerceAtMost(available.lastIndex))
    }
    PlaylistContent(
        title = playlist?.name ?: stringResource(R.string.favourites),
        rows = rows,
        reorderable = playlist != null,
        onBack = onBack,
        onPlay = ::play,
        onShuffle = { rows.mapNotNull(ListRow::track).takeIf { it.isNotEmpty() }?.let(player::shuffle) },
        onRemove = { i ->
            if (playlist != null) {
                app.lists.removeAt(playlist.id, i)
            } else {
                rows[i].track?.let { app.lists.setFavourite(it, false) }
            }
        },
        onMove = { from, to -> playlist?.let { app.lists.move(it.id, from, to) } },
        onRename = { renaming = true },
        onDelete = {
            playlist?.let { app.lists.delete(it.id) }
            onBack()
        },
    )
    if (renaming && playlist != null) {
        NameDialog(
            title = stringResource(R.string.rename_playlist),
            initial = playlist.name,
            confirm = stringResource(R.string.rename),
            onConfirm = {
                app.lists.rename(playlist.id, it)
                renaming = false
            },
            onDismiss = { renaming = false },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PlaylistContent(
    title: String,
    rows: List<ListRow>,
    reorderable: Boolean,
    onBack: () -> Unit,
    onPlay: (Int) -> Unit,
    onShuffle: () -> Unit,
    onRemove: (Int) -> Unit,
    onMove: (Int, Int) -> Unit,
    onRename: () -> Unit,
    onDelete: () -> Unit,
) {
    var menu by remember { mutableStateOf(false) }
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = { BackButton(onBack) },
                actions = {
                    if (reorderable) {
                        IconButton(onClick = { menu = true }) { Icon(Icons.Default.MoreVert, stringResource(R.string.more)) }
                        DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                            DropdownMenuItem(text = { Text(stringResource(R.string.rename)) }, onClick = {
                                menu = false
                                onRename()
                            })
                            DropdownMenuItem(text = { Text(stringResource(R.string.delete_playlist)) }, onClick = {
                                menu = false
                                onDelete()
                            })
                        }
                    }
                },
            )
        },
    ) { padding ->
        if (rows.isEmpty()) {
            Message(stringResource(if (reorderable) R.string.playlist_empty else R.string.favourites_empty), Modifier.padding(padding))
            return@Scaffold
        }
        // Rows by their place in [rows]; the order moves under the finger while dragging.
        var order by remember(rows) { mutableStateOf(rows.indices.toList()) }
        var dragged by remember { mutableStateOf<Int?>(null) }
        var offset by remember { mutableFloatStateOf(0f) }
        val list = rememberLazyListState()
        LazyColumn(Modifier.fillMaxSize(), state = list, contentPadding = padding) {
            item(key = "header") { Header(rows, onPlay = { onPlay(0) }, onShuffle) }
            items(order.size, key = { order[it] }) { position ->
                val i = order[position]
                val row = rows[i]
                val isDragged = dragged == i
                Row(
                    Modifier
                        .zIndex(if (isDragged) 1f else 0f)
                        .graphicsLayer { translationY = if (isDragged) offset else 0f },
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    if (reorderable) {
                        Icon(
                            Icons.Default.DragHandle,
                            stringResource(R.string.move),
                            Modifier.padding(start = 12.dp).size(24.dp).pointerInput(i) {
                                detectDragGestures(
                                    onDragStart = {
                                        dragged = i
                                        offset = 0f
                                    },
                                    onDrag = { change, amount ->
                                        change.consume()
                                        offset += amount.y
                                        val shown = list.layoutInfo.visibleItemsInfo
                                        val me = shown.firstOrNull { it.key == i } ?: return@detectDragGestures
                                        val middle = (me.offset + me.size / 2 + offset).toInt()
                                        val over = shown.firstOrNull { it.key is Int && it.key != i && middle in it.offset until it.offset + it.size }
                                            ?: return@detectDragGestures
                                        val from = order.indexOf(i)
                                        val to = order.indexOf(over.key as Int)
                                        order = order.toMutableList().apply { add(to, removeAt(from)) }
                                        // The row now sits where the other one was: keep it under the finger.
                                        offset += me.offset - over.offset
                                    },
                                    onDragEnd = {
                                        val to = order.indexOf(i)
                                        dragged = null
                                        offset = 0f
                                        if (to != i) onMove(i, to)
                                    },
                                    onDragCancel = {
                                        dragged = null
                                        offset = 0f
                                        order = rows.indices.toList()
                                    },
                                )
                            },
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Item(row, Modifier.weight(1f), onPlay = { onPlay(i) }, onRemove = { onRemove(i) })
                }
            }
        }
    }
}

@Composable
private fun Header(rows: List<ListRow>, onPlay: () -> Unit, onShuffle: () -> Unit) {
    val count = rows.size
    val duration = rows.sumOf { it.track?.durationMs ?: 0L }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
        Text(
            listOf(pluralStringResource(R.plurals.tracks_count, count, count), formatDuration(duration)).joinToString(" · "),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.size(12.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Button(onClick = onPlay, enabled = rows.any { it.track != null }) {
                Icon(Icons.Default.PlayArrow, null, Modifier.size(18.dp))
                Spacer(Modifier.width(8.dp))
                Text(stringResource(R.string.play))
            }
            FilledTonalButton(onClick = onShuffle, enabled = rows.any { it.track != null }) {
                Icon(Icons.Default.Shuffle, null, Modifier.size(18.dp))
                Spacer(Modifier.width(8.dp))
                Text(stringResource(R.string.shuffle))
            }
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun Item(row: ListRow, modifier: Modifier, onPlay: () -> Unit, onRemove: () -> Unit) {
    val track = row.track
    val openMenu = LocalTrackMenu.current
    ListItem(
        headlineContent = { Text(track?.title ?: row.listed.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = {
            Text(
                if (track == null) {
                    stringResource(R.string.not_on_phone)
                } else {
                    listOfNotNull(track.artist ?: stringResource(R.string.unknown_artist), track.album).joinToString(" · ")
                },
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        },
        leadingContent = { Cover(track?.albumId ?: -1, Modifier.size(48.dp)) },
        trailingContent = {
            IconButton(onClick = onRemove) { Icon(Icons.Default.Close, stringResource(R.string.remove_from_list)) }
        },
        modifier = modifier
            .alpha(if (track == null) 0.5f else 1f)
            .combinedClickable(enabled = track != null, onClick = onPlay, onLongClick = { track?.let(openMenu) }),
    )
}
