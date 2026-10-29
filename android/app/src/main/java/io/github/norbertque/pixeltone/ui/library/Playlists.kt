package io.github.norbertque.pixeltone.ui.library

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.QueueMusic
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.Lists
import io.github.norbertque.pixeltone.library.Playlist
import io.github.norbertque.pixeltone.library.fold

/** The library's Playlists tab: Favourites first, then the playlists (filtered by [query]). */
@Composable
fun PlaylistsTab(
    lists: Lists,
    query: String,
    onFavourites: () -> Unit,
    onPlaylist: (Long) -> Unit,
    onNew: () -> Unit,
) {
    val words = fold(query).split(' ').filter(String::isNotEmpty)
    val shown = lists.playlists.filter { p -> words.all { it in fold(p.name) } }
    LazyColumn(Modifier.fillMaxSize()) {
        if (words.isEmpty()) {
            item {
                ListRow(
                    icon = Icons.Default.Favorite,
                    title = stringResource(R.string.favourites),
                    count = lists.favourites.size,
                    onClick = onFavourites,
                )
            }
        }
        items(shown, key = Playlist::id) { playlist ->
            ListRow(Icons.AutoMirrored.Filled.QueueMusic, playlist.name, playlist.tracks.size) { onPlaylist(playlist.id) }
        }
        if (words.isEmpty()) {
            item {
                ListItem(
                    headlineContent = { Text(stringResource(R.string.new_playlist)) },
                    leadingContent = { Tile(Icons.Default.Add) },
                    modifier = Modifier.clickable(onClick = onNew),
                )
            }
        }
    }
}

@Composable
private fun ListRow(icon: ImageVector, title: String, count: Int, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = { Text(pluralStringResource(R.plurals.tracks_count, count, count)) },
        leadingContent = { Tile(icon) },
        modifier = Modifier.clickable(onClick = onClick),
    )
}

@Composable
private fun Tile(icon: ImageVector) {
    Box(
        Modifier.size(48.dp).clip(MaterialTheme.shapes.small).background(MaterialTheme.colorScheme.secondaryContainer),
        contentAlignment = Alignment.Center,
    ) {
        Icon(icon, null, tint = MaterialTheme.colorScheme.onSecondaryContainer)
    }
}

/** Which playlist [count] tracks go to, or a new one. */
@Composable
fun AddToPlaylistDialog(
    playlists: List<Playlist>,
    count: Int,
    onPick: (Long) -> Unit,
    onNew: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(pluralStringResource(R.plurals.add_tracks_to_playlist, count, count)) },
        text = {
            LazyColumn(Modifier.fillMaxWidth().heightIn(max = 360.dp)) {
                item {
                    ListItem(
                        headlineContent = { Text(stringResource(R.string.new_playlist)) },
                        leadingContent = { Tile(Icons.Default.Add) },
                        modifier = Modifier.clickable(onClick = onNew),
                    )
                }
                items(playlists, key = Playlist::id) { playlist ->
                    ListItem(
                        headlineContent = { Text(playlist.name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                        supportingContent = {
                            Text(pluralStringResource(R.plurals.tracks_count, playlist.tracks.size, playlist.tracks.size))
                        },
                        leadingContent = { Tile(Icons.AutoMirrored.Filled.QueueMusic) },
                        modifier = Modifier.clickable { onPick(playlist.id) },
                    )
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
}

/** A playlist's name, new or changed. */
@Composable
fun NameDialog(title: String, initial: String, confirm: String, onConfirm: (String) -> Unit, onDismiss: () -> Unit) {
    var name by rememberSaveable { mutableStateOf(initial) }
    val done = { if (name.isNotBlank()) onConfirm(name.trim()) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it.take(100) },
                    label = { Text(stringResource(R.string.playlist_name)) },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences, imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { done() }),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        },
        confirmButton = { TextButton(onClick = done, enabled = name.isNotBlank()) { Text(confirm) } },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
}
