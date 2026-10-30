package io.github.norbertque.pixeltone.ui.home

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Insights
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Shuffle
import androidx.compose.material3.Button
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.Album
import io.github.norbertque.pixeltone.library.Artist
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.library.Mix
import io.github.norbertque.pixeltone.library.listeningStats
import io.github.norbertque.pixeltone.library.recentlyAdded
import io.github.norbertque.pixeltone.library.recentlyPlayedAlbums
import io.github.norbertque.pixeltone.library.topArtists
import io.github.norbertque.pixeltone.ui.Loading
import io.github.norbertque.pixeltone.ui.library.AlbumCard
import io.github.norbertque.pixeltone.ui.library.Cover
import io.github.norbertque.pixeltone.ui.library.Mosaic
import io.github.norbertque.pixeltone.ui.library.rememberGenreColors
import io.github.norbertque.pixeltone.ui.stats.listeningTime
import io.github.norbertque.pixeltone.ui.theme.Emphasized
import java.util.Calendar
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** What Home shows, worked out from the library and the history. */
data class HomeData(
    val tracks: Int,
    val albums: Int,
    val artists: Int,
    val durationMs: Long,
    /** Today's mix. */
    val mix: List<Track>,
    val monthMs: Long,
    val recentlyPlayed: List<Album>,
    val topArtists: List<Pair<Artist, Int>>,
    val recentlyAdded: List<Album>,
    val genreMixes: List<Mix> = emptyList(),
)

@Composable
fun HomeScreen(
    app: PixeltoneApp,
    onAlbum: (String) -> Unit,
    onArtist: (String) -> Unit,
    onAlbums: () -> Unit,
    onStats: () -> Unit,
    onSettings: () -> Unit,
    onMix: (String) -> Unit,
) {
    val index by app.library.index.collectAsStateWithLifecycle()
    val listens by app.history.listens.collectAsStateWithLifecycle()
    val mixes by app.mixes.collectAsStateWithLifecycle()
    val data by produceState<HomeData?>(null, index, listens, mixes) {
        val library = index ?: return@produceState
        val made = mixes ?: return@produceState
        value = withContext(Dispatchers.Default) {
            val now = System.currentTimeMillis()
            val month = Calendar.getInstance().apply {
                set(Calendar.DAY_OF_MONTH, 1)
                set(Calendar.HOUR_OF_DAY, 0)
                set(Calendar.MINUTE, 0)
                set(Calendar.SECOND, 0)
            }.timeInMillis
            HomeData(
                tracks = library.tracks.size,
                albums = library.albums.size,
                artists = library.artists.size,
                durationMs = library.tracks.sumOf { it.durationMs },
                mix = made.daily.tracks,
                monthMs = listeningStats(listens, since = month).heardMs,
                recentlyPlayed = recentlyPlayedAlbums(library, listens),
                topArtists = topArtists(library, listens, since = month),
                recentlyAdded = recentlyAdded(library),
                genreMixes = made.genres,
            )
        }
    }
    val hour = remember { Calendar.getInstance().get(Calendar.HOUR_OF_DAY) }
    HomeContent(
        data = data,
        hour = hour,
        onShuffleAll = { index?.tracks?.takeIf { it.isNotEmpty() }?.let(app.player::shuffle) },
        onMix = { data?.mix?.takeIf { it.isNotEmpty() }?.let { app.player.play(it, 0) } },
        onAlbum = onAlbum,
        onArtist = onArtist,
        onAlbums = onAlbums,
        onStats = onStats,
        onSettings = onSettings,
        onOpenMix = onMix,
        onPlayMix = { mix -> mix.tracks.takeIf { it.isNotEmpty() }?.let { app.player.play(it, 0) } },
    )
}

@Composable
fun HomeContent(
    data: HomeData?,
    hour: Int,
    onShuffleAll: () -> Unit,
    onMix: () -> Unit,
    onAlbum: (String) -> Unit,
    onArtist: (String) -> Unit,
    onAlbums: () -> Unit,
    onStats: () -> Unit,
    onSettings: () -> Unit,
    onOpenMix: (String) -> Unit = {},
    onPlayMix: (Mix) -> Unit = {},
) {
    val greeting = stringResource(
        when {
            hour < 5 -> R.string.good_night
            hour < 12 -> R.string.good_morning
            hour < 18 -> R.string.good_afternoon
            else -> R.string.good_evening
        },
    )
    Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.surface) {
        LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = 24.dp)) {
            item {
                Column(Modifier.fillMaxWidth().statusBarsPadding().padding(start = 24.dp, end = 8.dp, top = 24.dp)) {
                    Row(verticalAlignment = Alignment.Top) {
                        Text(
                            greeting,
                            Modifier.weight(1f).padding(top = 8.dp),
                            style = MaterialTheme.typography.displayMedium,
                            fontWeight = Emphasized,
                        )
                        IconButton(onClick = onSettings) { Icon(Icons.Default.Settings, stringResource(R.string.settings)) }
                    }
                    if (data != null) {
                        Text(
                            listOf(
                                pluralStringResource(R.plurals.tracks_count, data.tracks, data.tracks),
                                pluralStringResource(R.plurals.albums_count, data.albums, data.albums),
                                pluralStringResource(R.plurals.artists_count, data.artists, data.artists),
                                listeningTime(data.durationMs),
                            ).joinToString(" · "),
                            Modifier.padding(top = 8.dp, end = 16.dp),
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Spacer(Modifier.height(16.dp))
                        Button(onClick = onShuffleAll, enabled = data.tracks > 0) {
                            Icon(Icons.Default.Shuffle, null, Modifier.size(18.dp))
                            Spacer(Modifier.width(8.dp))
                            Text(stringResource(R.string.shuffle_all))
                        }
                    }
                }
            }
            when {
                data == null -> item { Box(Modifier.fillMaxWidth().height(240.dp)) { Loading() } }
                data.tracks == 0 -> item {
                    Text(
                        stringResource(R.string.empty_library),
                        Modifier.fillMaxWidth().padding(32.dp),
                        textAlign = TextAlign.Center,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                else -> {
                    item { MixCard(data.mix, onOpen = { onOpenMix(Mix.DAILY) }, onPlay = onMix) }
                    item { StatsCard(data.monthMs, onStats) }
                    if (data.genreMixes.isNotEmpty()) {
                        item { SectionTitle(stringResource(R.string.genre_mixes)) }
                        item {
                            LazyRow(contentPadding = PaddingValues(horizontal = 24.dp), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                                items(data.genreMixes, key = Mix::key) { mix ->
                                    GenreMixCard(mix, onOpen = { onOpenMix(mix.key) }, onPlay = { onPlayMix(mix) })
                                }
                            }
                        }
                    }
                    if (data.recentlyPlayed.isNotEmpty()) {
                        item { SectionTitle(stringResource(R.string.recently_played)) }
                        item { AlbumRow(data.recentlyPlayed, onAlbum) }
                    }
                    if (data.topArtists.isNotEmpty()) {
                        item { SectionTitle(stringResource(R.string.your_artists_this_month)) }
                        item { ArtistRow(data.topArtists, onArtist) }
                    }
                    if (data.recentlyAdded.isNotEmpty()) {
                        item {
                            SectionTitle(stringResource(R.string.recently_added)) {
                                TextButton(onClick = onAlbums) {
                                    Icon(Icons.AutoMirrored.Filled.ArrowForward, null, Modifier.size(18.dp))
                                    Spacer(Modifier.width(8.dp))
                                    Text(stringResource(R.string.all_albums))
                                }
                            }
                        }
                        item { AlbumRow(data.recentlyAdded, onAlbum) }
                    }
                }
            }
        }
    }
}

/** Today's mix: four of its covers, what it is, and Play; the card opens it. */
@Composable
private fun MixCard(mix: List<Track>, onOpen: () -> Unit, onPlay: () -> Unit) {
    val glow = MaterialTheme.colorScheme.primaryContainer
    Surface(
        onClick = onOpen,
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        shape = MaterialTheme.shapes.extraLarge,
        color = MaterialTheme.colorScheme.surfaceContainerLow,
    ) {
        Column(
            Modifier
                .background(Brush.radialGradient(listOf(glow, Color.Transparent), center = Offset.Zero, radius = 900f))
                .padding(20.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Mosaic(mix.map { it.albumId }.distinct().take(4), Modifier.size(120.dp))
                Spacer(Modifier.width(20.dp))
                Column(Modifier.weight(1f)) {
                    Text(
                        stringResource(R.string.made_for_you).uppercase(),
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    Text(stringResource(R.string.todays_mix), style = MaterialTheme.typography.headlineSmall, fontWeight = Emphasized)
                }
            }
            Text(
                pluralStringResource(R.plurals.mix_text, mix.size, mix.size),
                Modifier.padding(vertical = 16.dp),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Button(onClick = onPlay, enabled = mix.isNotEmpty()) {
                Icon(Icons.Default.PlayArrow, null, Modifier.size(18.dp))
                Spacer(Modifier.width(8.dp))
                Text(stringResource(R.string.play))
            }
        }
    }
}

/** A genre's mix in the genre's colour: its covers with Play on them; the card opens it. */
@Composable
private fun GenreMixCard(mix: Mix, onOpen: () -> Unit, onPlay: () -> Unit) {
    val genre = mix.genre ?: return
    val (container, content) = rememberGenreColors(genre.name)
    Surface(onClick = onOpen, modifier = Modifier.width(168.dp), shape = MaterialTheme.shapes.extraLarge, color = container, contentColor = content) {
        Column(Modifier.padding(12.dp)) {
            Box {
                Mosaic(mix.tracks.map { it.albumId }.distinct().take(4), Modifier.fillMaxWidth().aspectRatio(1f))
                FilledIconButton(
                    onClick = onPlay,
                    modifier = Modifier.align(Alignment.BottomEnd).padding(8.dp),
                    colors = IconButtonDefaults.filledIconButtonColors(containerColor = content, contentColor = container),
                ) {
                    Icon(Icons.Default.PlayArrow, stringResource(R.string.play))
                }
            }
            Text(
                stringResource(R.string.genre_mix, genre.name),
                Modifier.padding(top = 12.dp, start = 4.dp, end = 4.dp),
                style = MaterialTheme.typography.titleMedium,
                fontWeight = Emphasized,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                pluralStringResource(R.plurals.tracks_count, mix.tracks.size, mix.tracks.size),
                Modifier.padding(horizontal = 4.dp).alpha(0.8f),
                style = MaterialTheme.typography.bodySmall,
            )
        }
    }
}

@Composable
private fun StatsCard(monthMs: Long, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        shape = MaterialTheme.shapes.extraLarge,
        color = MaterialTheme.colorScheme.surfaceContainerLow,
    ) {
        Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(
                Modifier.size(56.dp).clip(MaterialTheme.shapes.large).background(MaterialTheme.colorScheme.secondaryContainer),
                contentAlignment = Alignment.Center,
            ) {
                Icon(Icons.Default.Insights, null, tint = MaterialTheme.colorScheme.onSecondaryContainer)
            }
            Spacer(Modifier.width(16.dp))
            Column(Modifier.weight(1f)) {
                Text(stringResource(R.string.listening_stats), style = MaterialTheme.typography.titleMedium)
                Text(
                    if (monthMs > 0) {
                        stringResource(R.string.stats_this_month, listeningTime(monthMs))
                    } else {
                        stringResource(R.string.stats_teaser)
                    },
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null)
        }
    }
}

@Composable
private fun SectionTitle(text: String, action: (@Composable () -> Unit)? = null) {
    Row(Modifier.fillMaxWidth().padding(start = 24.dp, end = 12.dp, top = 28.dp, bottom = 12.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(text, Modifier.weight(1f), style = MaterialTheme.typography.headlineSmall, fontWeight = Emphasized)
        action?.invoke()
    }
}

@Composable
private fun AlbumRow(albums: List<Album>, onAlbum: (String) -> Unit) {
    LazyRow(contentPadding = PaddingValues(horizontal = 24.dp), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
        items(albums, key = Album::key) { album -> AlbumCard(album, onClick = { onAlbum(album.key) }, modifier = Modifier.width(160.dp)) }
    }
}

@Composable
private fun ArtistRow(artists: List<Pair<Artist, Int>>, onArtist: (String) -> Unit) {
    LazyRow(contentPadding = PaddingValues(horizontal = 24.dp), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
        items(artists, key = { it.first.key }) { (artist, plays) ->
            Column(
                Modifier.width(128.dp).clip(MaterialTheme.shapes.large).clickable { onArtist(artist.key) },
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                val cover = artist.albums.firstOrNull()?.coverId ?: artist.tracks.firstOrNull()?.albumId ?: -1
                Cover(cover, Modifier.size(128.dp), CircleShape)
                Text(
                    artist.name,
                    Modifier.padding(top = 8.dp),
                    style = MaterialTheme.typography.titleSmall,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    pluralStringResource(R.plurals.plays_count, plays, plays),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
