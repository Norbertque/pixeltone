package io.github.norbertque.pixeltone.ui.library

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Shuffle
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.Album
import io.github.norbertque.pixeltone.library.Genre
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.ui.BackButton
import io.github.norbertque.pixeltone.ui.Loading
import io.github.norbertque.pixeltone.ui.Message
import io.github.norbertque.pixeltone.ui.theme.Emphasized
import io.github.norbertque.pixeltone.ui.theme.ShapeXlPlus
import io.github.norbertque.pixeltone.ui.theme.genreColors
import androidx.compose.foundation.lazy.grid.items as gridItems

/** A genre's colours ([genreColors]) for the theme that shows, light or dark. */
@Composable
fun rememberGenreColors(name: String): Pair<Color, Color> {
    val dark = MaterialTheme.colorScheme.surface.luminance() < 0.5f
    return remember(name, dark) { genreColors(name, dark) }
}

/** The library's Genres tab: a card for each genre in its own colour, as on Windows. */
@Composable
fun GenresTab(genres: List<Genre>, query: String, onGenre: (String) -> Unit) {
    if (genres.isEmpty()) {
        Message(if (query.isBlank()) stringResource(R.string.no_genres) else stringResource(R.string.no_results, query.trim()))
        return
    }
    LazyVerticalGrid(
        columns = GridCells.Adaptive(160.dp),
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(16.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        gridItems(genres, key = Genre::key) { genre -> GenreCard(genre, onClick = { onGenre(genre.key) }) }
    }
}

@Composable
private fun GenreCard(genre: Genre, onClick: () -> Unit) {
    val (container, content) = rememberGenreColors(genre.name)
    Surface(
        onClick = onClick,
        modifier = Modifier.fillMaxWidth().height(112.dp),
        shape = MaterialTheme.shapes.extraLarge,
        color = container,
        contentColor = content,
    ) {
        Column(Modifier.padding(horizontal = 18.dp, vertical = 16.dp), verticalArrangement = Arrangement.SpaceBetween) {
            Text(genre.name, style = MaterialTheme.typography.titleLarge, fontWeight = Emphasized, maxLines = 2, overflow = TextOverflow.Ellipsis)
            Text(
                pluralStringResource(R.plurals.tracks_count, genre.tracks.size, genre.tracks.size),
                Modifier.alpha(0.8f),
                style = MaterialTheme.typography.labelMedium,
            )
        }
    }
}

@Composable
fun GenreScreen(
    key: String,
    onBack: () -> Unit,
    onAlbum: (String) -> Unit,
    onPlay: (List<Track>, Int) -> Unit,
    onShuffle: (List<Track>) -> Unit,
    vm: LibraryViewModel = viewModel(),
) {
    val index by vm.index.collectAsStateWithLifecycle()
    val library = index
    GenreContent(library?.genre(key), loading = library == null, onBack, onAlbum, onPlay, onShuffle)
}

/** A genre: its card, its albums, its tracks (as on Windows' genre page). */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GenreContent(
    genre: Genre?,
    loading: Boolean,
    onBack: () -> Unit,
    onAlbum: (String) -> Unit,
    onPlay: (List<Track>, Int) -> Unit,
    onShuffle: (List<Track>) -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = {},
                navigationIcon = { BackButton(onBack) },
                actions = { if (genre != null) ListMenuButton(TrackList(genre.name, genre.tracks)) },
            )
        },
    ) { padding ->
        when {
            loading -> Loading(Modifier.padding(padding))
            genre == null -> Message(stringResource(R.string.genre_gone), Modifier.padding(padding))
            else -> LazyColumn(Modifier.fillMaxSize(), contentPadding = padding) {
                item {
                    Hero(
                        overline = stringResource(R.string.genre),
                        title = genre.name,
                        details = counts(genre.albums.size, genre.tracks.size),
                        colors = rememberGenreColors(genre.name),
                        onPlay = { onPlay(genre.tracks, 0) },
                        onShuffle = { onShuffle(genre.tracks) },
                    )
                }
                if (genre.albums.isNotEmpty()) {
                    item { Heading(pluralStringResource(R.plurals.albums_count, genre.albums.size, genre.albums.size)) }
                    item {
                        LazyRow(contentPadding = PaddingValues(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                            items(genre.albums, key = Album::key) { album ->
                                AlbumCard(album, onClick = { onAlbum(album.key) }, Modifier.width(140.dp))
                            }
                        }
                    }
                }
                item { Heading(pluralStringResource(R.plurals.tracks_count, genre.tracks.size, genre.tracks.size)) }
                itemsIndexed(genre.tracks, key = { _, track -> track.id }) { i, track ->
                    TrackRow(track, onClick = { onPlay(genre.tracks, i) })
                }
            }
        }
    }
}

/**
 * The big card on top of a genre or a mix, as on Windows: what it is, its name, how much there
 * is, then Play and Shuffle, all in [colors] (a container and what goes on it).
 */
@Composable
internal fun Hero(
    overline: String,
    title: String,
    details: String,
    colors: Pair<Color, Color>,
    onPlay: () -> Unit,
    onShuffle: () -> Unit,
) {
    val (container, content) = colors
    Surface(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        shape = ShapeXlPlus,
        color = container,
        contentColor = content,
    ) {
        Column(Modifier.padding(24.dp)) {
            Text(overline.uppercase(), Modifier.alpha(0.8f), style = MaterialTheme.typography.labelLarge)
            Text(
                title,
                Modifier.padding(top = 4.dp),
                style = MaterialTheme.typography.displaySmall,
                fontWeight = Emphasized,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Text(details, Modifier.padding(top = 4.dp).alpha(0.8f), style = MaterialTheme.typography.bodyMedium)
            Row(Modifier.padding(top = 20.dp), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                Button(onClick = onPlay, colors = ButtonDefaults.buttonColors(containerColor = content, contentColor = container)) {
                    Icon(Icons.Default.PlayArrow, null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text(stringResource(R.string.play))
                }
                Button(onClick = onShuffle, colors = ButtonDefaults.buttonColors(containerColor = content.copy(alpha = 0.12f), contentColor = content)) {
                    Icon(Icons.Default.Shuffle, null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text(stringResource(R.string.shuffle))
                }
            }
        }
    }
}
