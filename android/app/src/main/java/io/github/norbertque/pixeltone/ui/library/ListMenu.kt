package io.github.norbertque.pixeltone.ui.library

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.PlaylistAdd
import androidx.compose.material.icons.automirrored.filled.QueueMusic
import androidx.compose.material.icons.filled.Album
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Shuffle
import androidx.compose.material.icons.filled.SkipNext
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.Track

/** A list of tracks with a menu of its own: an album, an artist, a genre, a mix, a folder. */
data class TrackList(
    val title: String,
    val tracks: List<Track>,
    /** Play and Shuffle head the menu where nothing else on screen offers them (an album card). */
    val withPlay: Boolean = false,
    /** The album the menu can open (an album card's). */
    val albumKey: String? = null,
)

/** Opens a list's menu; does nothing where none is set up. */
val LocalListMenu = staticCompositionLocalOf<(TrackList) -> Unit> { {} }

enum class ListAction { Play, Shuffle, PlayNext, AddToQueue, AddToPlaylist, Album }

/** What can be done with a whole list at once, as on Windows (`rowsMenu`, `albumMenu`). */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ListMenu(list: TrackList, onAction: (ListAction) -> Unit, onDismiss: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.navigationBarsPadding()) {
            ListItem(
                headlineContent = { Text(list.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                supportingContent = { Text(pluralStringResource(R.plurals.tracks_count, list.tracks.size, list.tracks.size)) },
                leadingContent = { Mosaic(list.tracks.map { it.albumId }.distinct().take(4), Modifier.size(48.dp)) },
            )
            HorizontalDivider()
            fun pick(action: ListAction) {
                onDismiss()
                onAction(action)
            }
            if (list.withPlay) {
                MenuAction(Icons.Default.PlayArrow, stringResource(R.string.play)) { pick(ListAction.Play) }
                MenuAction(Icons.Default.Shuffle, stringResource(R.string.shuffle)) { pick(ListAction.Shuffle) }
            }
            MenuAction(Icons.Default.SkipNext, stringResource(R.string.play_next)) { pick(ListAction.PlayNext) }
            MenuAction(Icons.AutoMirrored.Filled.QueueMusic, stringResource(R.string.add_to_queue)) { pick(ListAction.AddToQueue) }
            MenuAction(Icons.AutoMirrored.Filled.PlaylistAdd, stringResource(R.string.add_to_playlist)) { pick(ListAction.AddToPlaylist) }
            if (list.albumKey != null) MenuAction(Icons.Default.Album, stringResource(R.string.go_to_album)) { pick(ListAction.Album) }
        }
    }
}

/** The ⋮ that opens [list]'s menu. */
@Composable
fun ListMenuButton(list: TrackList) {
    val open = LocalListMenu.current
    IconButton(onClick = { open(list) }, enabled = list.tracks.isNotEmpty()) {
        Icon(Icons.Default.MoreVert, stringResource(R.string.more))
    }
}
