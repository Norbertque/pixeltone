package io.github.norbertque.pixeltone.ui.library

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.PlaylistAdd
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.Mix
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.ui.BackButton
import io.github.norbertque.pixeltone.ui.Loading
import io.github.norbertque.pixeltone.ui.Message
import io.github.norbertque.pixeltone.ui.stats.listeningTime

/** Today's mix, or a genre's. */
@Composable
fun mixName(mix: Mix): String = mix.genre?.let { stringResource(R.string.genre_mix, it.name) } ?: stringResource(R.string.todays_mix)

@Composable
fun MixScreen(app: PixeltoneApp, key: String, onBack: () -> Unit) {
    val mixes by app.mixes.collectAsStateWithLifecycle()
    MixContent(
        mix = mixes?.mix(key),
        loading = mixes == null,
        onBack = onBack,
        onPlay = app.player::play,
        onShuffle = app.player::shuffle,
        onSave = LocalAddToPlaylist.current,
    )
}

/** A mix's tracks under its card; they can be kept as a playlist of the user's. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MixContent(
    mix: Mix?,
    loading: Boolean,
    onBack: () -> Unit,
    onPlay: (List<Track>, Int) -> Unit,
    onShuffle: (List<Track>) -> Unit,
    onSave: (List<Track>) -> Unit,
) {
    Scaffold(topBar = { TopAppBar(title = {}, navigationIcon = { BackButton(onBack) }) }) { padding ->
        when {
            loading -> Loading(Modifier.padding(padding))
            mix == null || mix.tracks.isEmpty() -> Message(stringResource(R.string.mix_gone), Modifier.padding(padding))
            else -> LazyColumn(Modifier.fillMaxSize(), contentPadding = padding) {
                item {
                    val count = mix.tracks.size
                    Hero(
                        overline = stringResource(R.string.made_for_you),
                        title = mixName(mix),
                        details = listOf(
                            pluralStringResource(R.plurals.tracks_count, count, count),
                            listeningTime(mix.tracks.sumOf { it.durationMs }),
                            stringResource(R.string.new_every_day),
                        ).joinToString(" · "),
                        colors = mix.genre?.let { rememberGenreColors(it.name) }
                            ?: (MaterialTheme.colorScheme.primaryContainer to MaterialTheme.colorScheme.onPrimaryContainer),
                        onPlay = { onPlay(mix.tracks, 0) },
                        onShuffle = { onShuffle(mix.tracks) },
                        more = {
                            IconButton(onClick = { onSave(mix.tracks) }) {
                                Icon(Icons.AutoMirrored.Filled.PlaylistAdd, stringResource(R.string.save_as_playlist))
                            }
                        },
                    )
                }
                itemsIndexed(mix.tracks, key = { _, track -> track.id }) { i, track ->
                    TrackRow(track, onClick = { onPlay(mix.tracks, i) })
                }
            }
        }
    }
}
