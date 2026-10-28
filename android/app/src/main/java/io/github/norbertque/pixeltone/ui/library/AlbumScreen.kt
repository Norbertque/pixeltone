package io.github.norbertque.pixeltone.ui.library

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.Album
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.ui.BackButton
import io.github.norbertque.pixeltone.ui.Loading
import io.github.norbertque.pixeltone.ui.Message
import io.github.norbertque.pixeltone.ui.formatDuration

@Composable
fun AlbumScreen(
    key: String,
    onBack: () -> Unit,
    onArtist: (String) -> Unit,
    vm: LibraryViewModel = viewModel(),
) {
    val index by vm.index.collectAsStateWithLifecycle()
    val library = index
    val album = library?.album(key)
    val artistKey = album?.artist?.let { library.artistNamed(it)?.key }
    AlbumContent(
        album = album,
        loading = library == null,
        onBack = onBack,
        onArtist = artistKey?.let { { onArtist(it) } },
    )
}

/** An album's page; [onArtist] is null when its artist has no page. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AlbumContent(album: Album?, loading: Boolean, onBack: () -> Unit, onArtist: (() -> Unit)?) {
    Scaffold(topBar = { TopAppBar(title = {}, navigationIcon = { BackButton(onBack) }) }) { padding ->
        when {
            loading -> Loading(Modifier.padding(padding))
            album == null -> Message(stringResource(R.string.album_gone), Modifier.padding(padding))
            else -> LazyColumn(Modifier.fillMaxSize(), contentPadding = padding) {
                item { AlbumHeader(album, onArtist) }
                items(album.tracks, key = Track::id) {
                    // The artist only where it isn't the album's own (guests on a compilation).
                    val guest = it.artist != null && !it.artist.equals(album.artist, ignoreCase = true)
                    TrackRow(it, numbered = true, showArtist = guest, showAlbum = false)
                }
            }
        }
    }
}

@Composable
private fun AlbumHeader(album: Album, onArtist: (() -> Unit)?) {
    Column(
        Modifier.fillMaxWidth().padding(horizontal = 24.dp, vertical = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Cover(album.coverId, Modifier.fillMaxWidth(0.7f).aspectRatio(1f), MaterialTheme.shapes.large)
        Spacer(Modifier.height(20.dp))
        Text(album.title, style = MaterialTheme.typography.headlineSmall, textAlign = TextAlign.Center)
        val artist = Modifier.clip(MaterialTheme.shapes.small)
        Text(
            albumArtistName(album),
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.primary,
            textAlign = TextAlign.Center,
            modifier = (if (onArtist != null) artist.clickable(onClick = onArtist) else artist).padding(4.dp),
        )
        val tracks = album.tracks.size
        Text(
            listOfNotNull(
                album.year?.toString(),
                pluralStringResource(R.plurals.tracks_count, tracks, tracks),
                formatDuration(album.durationMs),
            ).joinToString(" · "),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(8.dp))
    }
}
