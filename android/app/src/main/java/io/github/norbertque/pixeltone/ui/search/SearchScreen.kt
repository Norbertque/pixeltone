package io.github.norbertque.pixeltone.ui.search

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.SearchResult
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.ui.library.AlbumCard
import io.github.norbertque.pixeltone.ui.library.ArtistRow
import io.github.norbertque.pixeltone.ui.library.LibraryViewModel
import io.github.norbertque.pixeltone.ui.library.TrackRow
import io.github.norbertque.pixeltone.ui.theme.Emphasized

@Composable
fun SearchScreen(
    onAlbum: (String) -> Unit,
    onArtist: (String) -> Unit,
    onPlay: (List<Track>, Int) -> Unit,
    vm: LibraryViewModel = viewModel(),
) {
    val results by vm.results.collectAsStateWithLifecycle()
    val query by vm.query.collectAsStateWithLifecycle()
    SearchContent(query, results.takeIf { query.isNotBlank() }, vm::search, onAlbum, onArtist, onPlay)
}

/** The Search tab: a field, then the tracks, albums and artists that match ([results] null before typing). */
@Composable
fun SearchContent(
    query: String,
    results: SearchResult?,
    onQuery: (String) -> Unit,
    onAlbum: (String) -> Unit,
    onArtist: (String) -> Unit,
    onPlay: (List<Track>, Int) -> Unit,
) {
    val keyboard = LocalSoftwareKeyboardController.current
    Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.surface) {
        LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = 24.dp)) {
            item {
                Column(Modifier.fillMaxWidth().statusBarsPadding().padding(start = 24.dp, end = 24.dp, top = 32.dp, bottom = 8.dp)) {
                    Text(stringResource(R.string.search), style = MaterialTheme.typography.displayMedium, fontWeight = Emphasized)
                    TextField(
                        value = query,
                        onValueChange = onQuery,
                        placeholder = { Text(stringResource(R.string.search_prompt)) },
                        leadingIcon = { Icon(Icons.Default.Search, null) },
                        trailingIcon = {
                            if (query.isNotEmpty()) {
                                IconButton(onClick = { onQuery("") }) { Icon(Icons.Default.Close, stringResource(R.string.close_search)) }
                            }
                        },
                        singleLine = true,
                        shape = CircleShape,
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                        keyboardActions = KeyboardActions(onSearch = { keyboard?.hide() }),
                        colors = TextFieldDefaults.colors(
                            focusedContainerColor = MaterialTheme.colorScheme.surfaceContainerHigh,
                            unfocusedContainerColor = MaterialTheme.colorScheme.surfaceContainerHigh,
                            focusedIndicatorColor = Color.Transparent,
                            unfocusedIndicatorColor = Color.Transparent,
                        ),
                        modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
                    )
                }
            }
            when {
                results == null -> item {
                    Text(
                        stringResource(R.string.search_start),
                        Modifier.fillMaxWidth().padding(32.dp),
                        textAlign = TextAlign.Center,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                results.tracks.isEmpty() && results.albums.isEmpty() && results.artists.isEmpty() -> item {
                    Text(
                        stringResource(R.string.no_results, query.trim()),
                        Modifier.fillMaxWidth().padding(32.dp),
                        textAlign = TextAlign.Center,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                else -> {
                    if (results.albums.isNotEmpty()) {
                        item { Heading(stringResource(R.string.tab_albums)) }
                        item {
                            LazyRow(contentPadding = PaddingValues(horizontal = 24.dp), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                                items(results.albums.take(20), key = { it.key }) { album ->
                                    AlbumCard(album, onClick = { onAlbum(album.key) }, modifier = Modifier.width(144.dp))
                                }
                            }
                        }
                    }
                    if (results.artists.isNotEmpty()) {
                        item { Heading(stringResource(R.string.tab_artists)) }
                        items(results.artists.take(8), key = { "artist-${it.key}" }) { artist -> ArtistRow(artist, onClick = { onArtist(artist.key) }) }
                    }
                    if (results.tracks.isNotEmpty()) {
                        item { Heading(stringResource(R.string.tab_tracks)) }
                        val tracks = results.tracks.take(100)
                        itemsIndexed(tracks, key = { _, t -> "track-${t.id}" }) { i, track -> TrackRow(track, onClick = { onPlay(tracks, i) }) }
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
        Modifier.padding(start = 24.dp, end = 24.dp, top = 24.dp, bottom = 8.dp),
        style = MaterialTheme.typography.titleLarge,
        fontWeight = Emphasized,
    )
}
