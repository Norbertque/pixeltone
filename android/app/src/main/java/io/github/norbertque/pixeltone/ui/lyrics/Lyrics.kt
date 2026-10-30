package io.github.norbertque.pixeltone.ui.lyrics

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.trackUri
import io.github.norbertque.pixeltone.lyrics.LyricLine
import io.github.norbertque.pixeltone.lyrics.Lyrics
import io.github.norbertque.pixeltone.lyrics.LyricsRequest
import io.github.norbertque.pixeltone.ui.Loading
import io.github.norbertque.pixeltone.ui.Message

/** Lyrics as they are looked for: still looking, found, or not there. */
sealed interface LyricsState {
    data object Looking : LyricsState

    data class Found(val lyrics: Lyrics) : LyricsState

    data object Missing : LyricsState
}

/** What Now Playing's panel shows: the lyrics of what plays now, as they're found. */
class LyricsNow(val title: String, val artist: String?, val state: LyricsState, val online: Boolean, val positionMs: Long)

/** The lyrics of what plays now, on the phone or on a PC, looked for again when the track changes. */
@Composable
fun rememberLyricsNow(app: PixeltoneApp): LyricsNow {
    val playback by app.player.state.collectAsStateWithLifecycle()
    val position by app.player.positionMs.collectAsStateWithLifecycle(initialValue = 0L)
    val index by app.library.index.collectAsStateWithLifecycle()
    val online by app.lyrics.online.collectAsStateWithLifecycle()
    val entry = playback.current
    val track = entry?.trackId?.takeIf { it >= 0 }?.let { index?.track(it) }
    val request = entry?.let {
        LyricsRequest(
            key = track?.fileKey ?: "pc:${it.title}|${it.artist}|${it.album}|${playback.durationMs / 1000}",
            uri = track?.let { t -> trackUri(t.id) },
            title = it.title,
            artist = it.artist,
            album = it.album,
            durationMs = track?.durationMs ?: playback.durationMs,
        )
    }
    var state by remember(request?.key) { mutableStateOf<LyricsState>(LyricsState.Looking) }
    LaunchedEffect(request?.key, online) {
        state = request?.let { app.lyrics.find(it) }?.let { LyricsState.Found(it) } ?: LyricsState.Missing
    }
    return LyricsNow(entry?.title.orEmpty(), entry?.artist, state, online, position)
}

/** The lyrics themselves: synced ones follow [positionMs] (a tap on a line seeks), plain ones scroll. */
@Composable
fun LyricsBody(state: LyricsState, online: Boolean, positionMs: Long, onSeek: (Long) -> Unit, modifier: Modifier = Modifier) {
    Box(modifier.fillMaxSize()) {
        when (state) {
            LyricsState.Looking -> Loading()
            LyricsState.Missing -> Message(stringResource(if (online) R.string.lyrics_missing else R.string.lyrics_missing_offline))
            is LyricsState.Found -> {
                val lyrics = state.lyrics
                when {
                    lyrics.instrumental -> Message(stringResource(R.string.lyrics_instrumental))
                    lyrics.synced -> Synced(lyrics.lines, lyrics.source, positionMs, onSeek)
                    else -> Plain(lyrics.lines, lyrics.source)
                }
            }
        }
    }
}

@Composable
private fun Synced(lines: List<LyricLine>, source: Lyrics.Source, positionMs: Long, onSeek: (Long) -> Unit) {
    val current = lines.indexOfLast { (it.timeMs ?: 0) <= positionMs }
    val list = rememberLazyListState()
    // Follow the song, unless the user is reading elsewhere right now.
    LaunchedEffect(current) {
        if (current >= 0 && !list.isScrollInProgress) list.animateScrollToItem((current - 2).coerceAtLeast(0))
    }
    LazyColumn(Modifier.fillMaxSize(), state = list, contentPadding = PaddingValues(24.dp, 16.dp, 24.dp, 240.dp)) {
        itemsIndexed(lines) { i, line ->
            val now = i == current
            Text(
                line.text.ifEmpty { "♪" },
                Modifier.fillMaxWidth().clickable { line.timeMs?.let(onSeek) }.padding(vertical = 8.dp),
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = if (now) FontWeight.Bold else FontWeight.Normal,
                color = if (now) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = if (i < current) 0.5f else 0.8f),
            )
        }
        item { Source(source) }
    }
}

@Composable
private fun Plain(lines: List<LyricLine>, source: Lyrics.Source) {
    LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(24.dp, 16.dp)) {
        itemsIndexed(lines) { _, line ->
            Text(line.text, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.padding(vertical = 2.dp))
        }
        item { Source(source) }
    }
}

@Composable
private fun Source(source: Lyrics.Source) {
    Box(Modifier.fillMaxWidth().padding(top = 24.dp), contentAlignment = Alignment.CenterStart) {
        Text(
            stringResource(if (source == Lyrics.Source.LRCLIB) R.string.lyrics_from_lrclib else R.string.lyrics_from_file),
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
