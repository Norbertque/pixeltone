package io.github.norbertque.pixeltone.ui.library

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.PlaylistAdd
import androidx.compose.material.icons.automirrored.filled.QueueMusic
import androidx.compose.material.icons.filled.Album
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material.icons.filled.FavoriteBorder
import androidx.compose.material.icons.filled.FolderOpen
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.SkipNext
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.Track

/** Opens a track's menu (a long press on its row); does nothing where none is set up. */
val LocalTrackMenu = staticCompositionLocalOf<(Track) -> Unit> { {} }

/** Asks which playlist the tracks go to; does nothing where it isn't set up. */
val LocalAddToPlaylist = staticCompositionLocalOf<(List<Track>) -> Unit> { {} }

enum class TrackAction { PlayNext, AddToQueue, AddToPlaylist, Favourite, Album, Artist, Folder }

/** What can be done with one track; [hasAlbum] / [hasArtist]: whether those pages exist. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TrackMenu(
    track: Track,
    favourite: Boolean,
    hasAlbum: Boolean,
    hasArtist: Boolean,
    onAction: (TrackAction) -> Unit,
    onDismiss: () -> Unit,
) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.navigationBarsPadding()) {
            ListItem(
                headlineContent = { Text(track.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                supportingContent = {
                    Text(
                        listOfNotNull(track.artist ?: stringResource(R.string.unknown_artist), track.album).joinToString(" · "),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                },
                leadingContent = { Cover(track.albumId, Modifier.size(48.dp)) },
            )
            HorizontalDivider()
            fun pick(action: TrackAction) {
                onDismiss()
                onAction(action)
            }
            MenuAction(Icons.Default.SkipNext, stringResource(R.string.play_next)) { pick(TrackAction.PlayNext) }
            MenuAction(Icons.AutoMirrored.Filled.QueueMusic, stringResource(R.string.add_to_queue)) { pick(TrackAction.AddToQueue) }
            MenuAction(Icons.AutoMirrored.Filled.PlaylistAdd, stringResource(R.string.add_to_playlist)) { pick(TrackAction.AddToPlaylist) }
            if (favourite) {
                MenuAction(Icons.Default.Favorite, stringResource(R.string.unfavourite)) { pick(TrackAction.Favourite) }
            } else {
                MenuAction(Icons.Default.FavoriteBorder, stringResource(R.string.favourite)) { pick(TrackAction.Favourite) }
            }
            if (hasAlbum) MenuAction(Icons.Default.Album, stringResource(R.string.go_to_album)) { pick(TrackAction.Album) }
            if (hasArtist) MenuAction(Icons.Default.Person, stringResource(R.string.go_to_artist)) { pick(TrackAction.Artist) }
            MenuAction(Icons.Default.FolderOpen, stringResource(R.string.show_in_folder)) { pick(TrackAction.Folder) }
        }
    }
}

@Composable
internal fun MenuAction(icon: ImageVector, label: String, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(label) },
        leadingContent = { Icon(icon, null) },
        modifier = Modifier.clickable(onClick = onClick),
    )
}
