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
import io.github.norbertque.pixeltone.ui.theme.Emphasized
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.Alignment
import androidx.compose.material3.Icon
import androidx.compose.material3.Button
import androidx.compose.material.icons.filled.Shuffle
import androidx.compose.material.icons.Icons
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.Column
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
                title = {},
                navigationIcon = { BackButton(onBack) },
                actions = { if (artist != null) ListMenuButton(TrackList(artist.name, artist.tracks)) },
            )
        },
    ) { padding ->
        when {
            loading -> Loading(Modifier.padding(padding))
            artist == null -> Message(stringResource(R.string.artist_gone), Modifier.padding(padding))
            else -> LazyColumn(Modifier.fillMaxSize(), contentPadding = padding) {
                item {
                    // Their picture round, their name big, as on Windows.
                    Column(Modifier.fillMaxWidth().padding(horizontal = 24.dp, vertical = 8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                        val cover = artist.albums.firstOrNull()?.coverId ?: artist.tracks.firstOrNull()?.albumId ?: -1
                        Cover(cover, Modifier.size(200.dp), CircleShape)
                        Text(
                            artist.name,
                            Modifier.padding(top = 20.dp),
                            style = MaterialTheme.typography.displaySmall,
                            fontWeight = Emphasized,
                            textAlign = TextAlign.Center,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                        Button(onClick = { onPlay(artist.tracks.shuffled(), 0) }, enabled = artist.tracks.isNotEmpty(), modifier = Modifier.padding(top = 16.dp)) {
                            Icon(Icons.Default.Shuffle, null, Modifier.size(18.dp))
                            Spacer(Modifier.width(8.dp))
                            Text(stringResource(R.string.shuffle))
                        }
                    }
                }
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
internal fun Heading(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.titleLarge,
        fontWeight = Emphasized,
        modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 24.dp, bottom = 8.dp),
    )
}
