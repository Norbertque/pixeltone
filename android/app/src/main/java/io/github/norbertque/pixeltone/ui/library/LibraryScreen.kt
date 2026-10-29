package io.github.norbertque.pixeltone.ui.library

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Devices
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.Album
import io.github.norbertque.pixeltone.library.Artist
import io.github.norbertque.pixeltone.library.SearchResult
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.ui.Loading
import io.github.norbertque.pixeltone.ui.Message
import io.github.norbertque.pixeltone.library.Lists
import androidx.compose.foundation.lazy.grid.items as gridItems

private val TABS = listOf(R.string.tab_tracks, R.string.tab_albums, R.string.tab_artists, R.string.tab_playlists)

@Composable
fun LibraryScreen(
    onAlbum: (String) -> Unit,
    onArtist: (String) -> Unit,
    onPlay: (List<Track>, Int) -> Unit,
    onPlayOn: () -> Unit,
    onSettings: () -> Unit = {},
    lists: Lists = Lists(),
    onFavourites: () -> Unit = {},
    onPlaylist: (Long) -> Unit = {},
    onNewPlaylist: () -> Unit = {},
    vm: LibraryViewModel = viewModel(),
) {
    val results by vm.results.collectAsStateWithLifecycle()
    val query by vm.query.collectAsStateWithLifecycle()
    var tab by rememberSaveable { mutableIntStateOf(0) }
    var searching by rememberSaveable { mutableStateOf(false) }
    LibraryContent(
        results = results,
        query = query,
        tab = tab,
        searching = searching,
        onTab = { tab = it },
        onSearching = {
            searching = it
            if (!it) vm.search("")
        },
        onQuery = vm::search,
        onAlbum = onAlbum,
        onArtist = onArtist,
        onPlay = onPlay,
        onPlayOn = onPlayOn,
        onSettings = onSettings,
        lists = lists,
        onFavourites = onFavourites,
        onPlaylist = onPlaylist,
        onNewPlaylist = onNewPlaylist,
    )
}

/** The library tabs; [results] is null while the library is read. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LibraryContent(
    results: SearchResult?,
    query: String,
    tab: Int,
    searching: Boolean,
    onTab: (Int) -> Unit,
    onSearching: (Boolean) -> Unit,
    onQuery: (String) -> Unit,
    onAlbum: (String) -> Unit,
    onArtist: (String) -> Unit,
    onPlay: (List<Track>, Int) -> Unit,
    onPlayOn: () -> Unit = {},
    onSettings: () -> Unit = {},
    lists: Lists = Lists(),
    onFavourites: () -> Unit = {},
    onPlaylist: (Long) -> Unit = {},
    onNewPlaylist: () -> Unit = {},
) {
    BackHandler(enabled = searching) { onSearching(false) }
    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    if (searching) SearchField(query, onQuery) else Text(stringResource(R.string.app_name))
                },
                actions = {
                    if (searching) {
                        IconButton(onClick = { onSearching(false) }) {
                            Icon(Icons.Default.Close, stringResource(R.string.close_search))
                        }
                    } else {
                        IconButton(onClick = { onSearching(true) }) {
                            Icon(Icons.Default.Search, stringResource(R.string.search))
                        }
                        IconButton(onClick = onPlayOn) {
                            Icon(Icons.Default.Devices, stringResource(R.string.play_on))
                        }
                        IconButton(onClick = onSettings) {
                            Icon(Icons.Default.Settings, stringResource(R.string.settings))
                        }
                    }
                },
            )
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            PrimaryTabRow(selectedTabIndex = tab) {
                TABS.forEachIndexed { i, label ->
                    Tab(selected = tab == i, onClick = { onTab(i) }, text = { Text(stringResource(label)) })
                }
            }
            when {
                // Playlists can be looked at (and made) before the library is read, and without tracks.
                tab == 3 -> PlaylistsTab(lists, query, onFavourites, onPlaylist, onNewPlaylist)
                results == null -> Loading()
                query.isBlank() && results.tracks.isEmpty() -> Message(stringResource(R.string.empty_library))
                tab == 0 -> Tracks(results.tracks, query, onPlay)
                tab == 1 -> Albums(results.albums, query, onAlbum)
                else -> Artists(results.artists, query, onArtist)
            }
        }
    }
}

@Composable
private fun Tracks(tracks: List<Track>, query: String, onPlay: (List<Track>, Int) -> Unit) {
    if (tracks.isEmpty()) {
        NoMatch(query)
        return
    }
    LazyColumn(Modifier.fillMaxSize()) {
        itemsIndexed(tracks, key = { _, track -> track.id }) { i, track -> TrackRow(track, onClick = { onPlay(tracks, i) }) }
    }
}

@Composable
private fun Albums(albums: List<Album>, query: String, onAlbum: (String) -> Unit) {
    if (albums.isEmpty()) {
        NoMatch(query)
        return
    }
    LazyVerticalGrid(
        columns = GridCells.Adaptive(152.dp),
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(16.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        gridItems(albums, key = Album::key) { album -> AlbumCard(album, onClick = { onAlbum(album.key) }) }
    }
}

@Composable
private fun Artists(artists: List<Artist>, query: String, onArtist: (String) -> Unit) {
    if (artists.isEmpty()) {
        NoMatch(query)
        return
    }
    LazyColumn(Modifier.fillMaxSize()) {
        items(artists, key = Artist::key) { artist -> ArtistRow(artist, onClick = { onArtist(artist.key) }) }
    }
}

@Composable
private fun NoMatch(query: String) {
    if (query.isNotBlank()) Message(stringResource(R.string.no_results, query.trim()))
}

@Composable
private fun SearchField(query: String, onQuery: (String) -> Unit) {
    val focus = remember { FocusRequester() }
    val keyboard = LocalSoftwareKeyboardController.current
    LaunchedEffect(Unit) { focus.requestFocus() }
    TextField(
        value = query,
        onValueChange = onQuery,
        placeholder = { Text(stringResource(R.string.search_hint)) },
        singleLine = true,
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
        keyboardActions = KeyboardActions(onSearch = { keyboard?.hide() }),
        colors = TextFieldDefaults.colors(
            focusedContainerColor = Color.Transparent,
            unfocusedContainerColor = Color.Transparent,
            focusedIndicatorColor = Color.Transparent,
            unfocusedIndicatorColor = Color.Transparent,
        ),
        modifier = Modifier.fillMaxWidth().focusRequester(focus),
    )
}
