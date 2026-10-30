package io.github.norbertque.pixeltone.ui.stats

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.ListedTrack
import io.github.norbertque.pixeltone.library.ListeningStats
import io.github.norbertque.pixeltone.library.listeningStats
import io.github.norbertque.pixeltone.ui.BackButton
import io.github.norbertque.pixeltone.ui.Message

private val PERIODS = listOf(R.string.stats_week, R.string.stats_month, R.string.stats_all)
private const val DAY_MS = 86_400_000L

@Composable
fun StatsScreen(app: PixeltoneApp, onBack: () -> Unit) {
    val listens by app.history.listens.collectAsStateWithLifecycle()
    val index by app.library.index.collectAsStateWithLifecycle()
    var period by rememberSaveable { mutableIntStateOf(1) }
    val now = remember(listens) { System.currentTimeMillis() }
    val stats = remember(listens, period, index) {
        val since = when (period) {
            0 -> now - 7 * DAY_MS
            1 -> now - 30 * DAY_MS
            else -> null
        }
        listeningStats(listens, since, artistsOf = { index?.artistNames(it) ?: listOf(it) })
    }
    StatsContent(
        stats = stats,
        period = period,
        now = now,
        onPeriod = { period = it },
        onBack = onBack,
        onTrack = { listed -> index?.trackByFile(listed.file)?.let { app.player.play(listOf(it), 0) } },
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StatsContent(
    stats: ListeningStats,
    period: Int,
    now: Long,
    onPeriod: (Int) -> Unit,
    onBack: () -> Unit,
    onTrack: (ListedTrack) -> Unit,
) {
    Scaffold(
        topBar = { TopAppBar(title = { Text(stringResource(R.string.listening_stats)) }, navigationIcon = { BackButton(onBack) }) },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
                PERIODS.forEachIndexed { i, label ->
                    SegmentedButton(
                        selected = period == i,
                        onClick = { onPeriod(i) },
                        shape = SegmentedButtonDefaults.itemShape(i, PERIODS.size),
                    ) { Text(stringResource(label)) }
                }
            }
            if (stats.plays == 0) {
                Message(stringResource(R.string.stats_empty))
                return@Column
            }
            LazyColumn(Modifier.fillMaxSize()) {
                item {
                    Column(Modifier.fillMaxWidth().padding(16.dp)) {
                        Text(listeningTime(stats.heardMs), style = MaterialTheme.typography.displaySmall)
                        Text(
                            pluralStringResource(R.plurals.plays_count, stats.plays, stats.plays),
                            style = MaterialTheme.typography.titleMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                ranked(R.string.stats_top_tracks, stats.tracks.map { Triple(it.item.title, it.item.artist, it.plays) }) { i ->
                    onTrack(stats.tracks[i].item)
                }
                ranked(R.string.stats_top_artists, stats.artists.map { Triple(it.item, null, it.plays) }, null)
                ranked(R.string.stats_top_albums, stats.albums.map { Triple(it.item.title, it.item.artist, it.plays) }, null)
                item { Section(stringResource(R.string.stats_recent)) }
                itemsIndexed(stats.recent) { _, listen ->
                    ListItem(
                        headlineContent = { Text(listen.track.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                        supportingContent = {
                            Text(
                                listOfNotNull(listen.track.artist, ago(now - listen.startedAt)).joinToString(" · "),
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        },
                        modifier = Modifier.clickable { onTrack(listen.track) },
                    )
                }
            }
        }
    }
}

private fun LazyListScope.ranked(title: Int, rows: List<Triple<String, String?, Int>>, onClick: ((Int) -> Unit)?) {
    if (rows.isEmpty()) return
    item { Section(stringResource(title)) }
    itemsIndexed(rows) { i, (name, by, plays) ->
        ListItem(
            headlineContent = { Text(name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
            supportingContent = by?.let { { Text(it, maxLines = 1, overflow = TextOverflow.Ellipsis) } },
            leadingContent = {
                Text(
                    "${i + 1}",
                    Modifier.width(28.dp),
                    textAlign = TextAlign.End,
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = { Text(pluralStringResource(R.plurals.plays_count, plays, plays), style = MaterialTheme.typography.labelMedium) },
            modifier = if (onClick != null) Modifier.clickable { onClick(i) } else Modifier,
        )
    }
}

@Composable
private fun Section(text: String) {
    Text(
        text,
        Modifier.padding(start = 16.dp, end = 16.dp, top = 20.dp, bottom = 4.dp),
        style = MaterialTheme.typography.titleMedium,
    )
}

/** "12 h 5 min", "45 min". */
internal fun listeningTime(ms: Long): String {
    val minutes = ms / 60_000
    return if (minutes >= 60) "${minutes / 60} h ${minutes % 60} min" else "$minutes min"
}

/** "just now", "5 min ago", "3 h ago", "2 days ago". */
internal fun ago(ms: Long): String {
    val minutes = ms / 60_000
    return when {
        minutes < 1 -> "just now"
        minutes < 60 -> "$minutes min ago"
        minutes < 48 * 60 -> "${minutes / 60} h ago"
        else -> "${minutes / (24 * 60)} days ago"
    }
}
