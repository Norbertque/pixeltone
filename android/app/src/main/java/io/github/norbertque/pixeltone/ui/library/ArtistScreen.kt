package io.github.norbertque.pixeltone.ui.library

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.Album
import io.github.norbertque.pixeltone.library.Artist
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.ui.BackButton
import io.github.norbertque.pixeltone.ui.Loading
import io.github.norbertque.pixeltone.ui.Message

@Composable
fun ArtistScreen(
    key: String,
    onBack: () -> Unit,
    onAlbum: (String) -> Unit,
    onPlay: (List<Track>, Int) -> Unit,
    vm: LibraryViewModel = viewModel(),
) {
    val index by vm.index.collectAsStateWithLifecycle()
    val library = index
    ArtistContent(artist = library?.artist(key), loading = library == null, onBack = onBack, onAlbum = onAlbum, onPlay = onPlay)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ArtistContent(
    artist: Artist?,
    loading: Boolean,
    onBack: () -> Unit,
    onAlbum: (String) -> Unit,
    onPlay: (List<Track>, Int) -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(artist?.name.orEmpty(), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        when {
            loading -> Loading(Modifier.padding(padding))
            artist == null -> Message(stringResource(R.string.artist_gone), Modifier.padding(padding))
            else -> LazyColumn(Modifier.fillMaxSize(), contentPadding = padding) {
                if (artist.albums.isNotEmpty()) {
                    item { Heading(pluralStringResource(R.plurals.albums_count, artist.albums.size, artist.albums.size)) }
                    item {
                        LazyRow(
                            contentPadding = PaddingValues(horizontal = 16.dp),
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            items(artist.albums, key = Album::key) { album ->
                                AlbumCard(album, onClick = { onAlbum(album.key) }, Modifier.width(140.dp))
                            }
                        }
                    }
                }
                if (artist.tracks.isNotEmpty()) {
                    item { Heading(pluralStringResource(R.plurals.tracks_count, artist.tracks.size, artist.tracks.size)) }
                    itemsIndexed(artist.tracks, key = { _, track -> track.id }) { i, track ->
                        TrackRow(track, onClick = { onPlay(artist.tracks, i) })
                    }
                }
            }
        }
    }
}

@Composable
private fun Heading(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.titleMedium,
        modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 20.dp, bottom = 8.dp),
    )
}
