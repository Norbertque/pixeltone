package io.github.norbertque.pixeltone.ui.jam

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.LibraryIndex
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.remote.PcLibraryAlbum
import io.github.norbertque.pixeltone.remote.PcLibraryAlbumDetail
import io.github.norbertque.pixeltone.remote.PcLibraryResults
import io.github.norbertque.pixeltone.remote.PcLibraryTrack
import io.github.norbertque.pixeltone.ui.Loading
import io.github.norbertque.pixeltone.ui.Message
import io.github.norbertque.pixeltone.ui.formatDuration
import io.github.norbertque.pixeltone.ui.library.Cover
import kotlinx.coroutines.delay

/** Where tracks for the jam come from: the PC's own library, by id. */
interface PcLibrary {
    suspend fun search(query: String): PcLibraryResults

    suspend fun albums(): List<PcLibraryAlbum>

    suspend fun album(id: Long): PcLibraryAlbumDetail
}

/**
 * Adding tracks to a jam: from the PC's library (they're there already), or from this phone
 * (sent as copies).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AddTracksSheet(
    pcName: String,
    pc: PcLibrary,
    coverOf: (String) -> Any,
    phone: LibraryIndex?,
    phoneAllowed: Boolean,
    onAllowPhone: () -> Unit,
    onAddFromPc: (List<Long>) -> Unit,
    onSendFromPhone: (List<Track>) -> Unit,
    onDismiss: () -> Unit,
) {
    var tab by rememberSaveable { mutableIntStateOf(0) }
    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        Column(Modifier.fillMaxHeight(0.92f)) {
            PrimaryTabRow(selectedTabIndex = tab) {
                Tab(selected = tab == 0, onClick = { tab = 0 }, text = { Text(stringResource(R.string.jam_on_pc, pcName), maxLines = 1) })
                Tab(selected = tab == 1, onClick = { tab = 1 }, text = { Text(stringResource(R.string.jam_on_phone)) })
            }
            when (tab) {
                0 -> PcTab(pc, coverOf, onAddFromPc)
                else -> PhoneTab(phone, phoneAllowed, onAllowPhone, onSendFromPhone)
            }
        }
    }
}

@Composable
private fun SearchField(query: String, onChange: (String) -> Unit) {
    OutlinedTextField(
        value = query,
        onValueChange = onChange,
        placeholder = { Text(stringResource(R.string.search)) },
        leadingIcon = { Icon(Icons.Default.Search, null) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth().padding(16.dp, 12.dp),
    )
}

@Composable
private fun PcTab(pc: PcLibrary, coverOf: (String) -> Any, onAdd: (List<Long>) -> Unit) {
    var query by rememberSaveable { mutableStateOf("") }
    var album by remember { mutableStateOf<PcLibraryAlbum?>(null) }
    val added = remember { mutableStateListOf<Long>() }
    val add = { ids: List<Long> ->
        val fresh = ids.filterNot(added::contains)
        if (fresh.isNotEmpty()) {
            added += fresh
            onAdd(fresh)
        }
    }
    val opened = album
    if (opened != null) {
        PcAlbum(pc, opened, coverOf, added, add, onBack = { album = null })
        return
    }
    SearchField(query) { query = it }
    var albums by remember { mutableStateOf<List<PcLibraryAlbum>?>(null) }
    var found by remember { mutableStateOf<PcLibraryResults?>(null) }
    var failed by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(query) {
        failed = null
        try {
            if (query.isBlank()) {
                if (albums == null) albums = pc.albums()
                found = null
            } else {
                delay(250) // typing
                found = pc.search(query.trim())
            }
        } catch (e: java.io.IOException) {
            failed = e.message
        }
    }
    when {
        failed != null -> Message(failed.orEmpty())
        query.isBlank() && albums == null -> Loading()
        query.isBlank() -> LazyColumn {
            items(albums.orEmpty(), key = { it.id }) { a -> AlbumItem(a, coverOf) { album = a } }
        }
        found == null -> Loading()
        found!!.tracks.isEmpty() && found!!.albums.isEmpty() -> Message(stringResource(R.string.no_results))
        else -> LazyColumn {
            items(found!!.albums, key = { "album-${it.id}" }) { a -> AlbumItem(a, coverOf) { album = a } }
            items(found!!.tracks, key = { "track-${it.id}" }) { t -> PcTrackItem(t, coverOf, t.id in added) { add(listOf(t.id)) } }
        }
    }
}

@Composable
private fun PcAlbum(
    pc: PcLibrary,
    album: PcLibraryAlbum,
    coverOf: (String) -> Any,
    added: List<Long>,
    onAdd: (List<Long>) -> Unit,
    onBack: () -> Unit,
) {
    var detail by remember(album.id) { mutableStateOf<PcLibraryAlbumDetail?>(null) }
    var failed by remember(album.id) { mutableStateOf<String?>(null) }
    LaunchedEffect(album.id) {
        try {
            detail = pc.album(album.id)
        } catch (e: java.io.IOException) {
            failed = e.message
        }
    }
    ListItem(
        headlineContent = { Text(album.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = { album.artist?.let { Text(it, maxLines = 1) } },
        leadingContent = {
            IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, stringResource(R.string.back)) }
        },
        trailingContent = {
            val tracks = detail?.tracks.orEmpty()
            TextButton(onClick = { onAdd(tracks.map { it.id }) }, enabled = tracks.any { it.id !in added }) {
                Text(stringResource(R.string.jam_add_all))
            }
        },
    )
    when {
        failed != null -> Message(failed.orEmpty())
        detail == null -> Loading()
        else -> LazyColumn {
            items(detail!!.tracks, key = { it.id }) { t -> PcTrackItem(t, coverOf, t.id in added) { onAdd(listOf(t.id)) } }
        }
    }
}

@Composable
private fun AlbumItem(album: PcLibraryAlbum, coverOf: (String) -> Any, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(album.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = {
            Text(
                listOfNotNull(album.artist, pluralStringResource(R.plurals.tracks_count, album.trackCount, album.trackCount)).joinToString(" · "),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        },
        leadingContent = { Cover(album.coverKey?.let(coverOf), Modifier.size(48.dp)) },
        modifier = Modifier.clickable(onClick = onClick),
    )
}

@Composable
private fun PcTrackItem(track: PcLibraryTrack, coverOf: (String) -> Any, added: Boolean, onAdd: () -> Unit) {
    ListItem(
        headlineContent = { Text(track.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = {
            Text(listOfNotNull(track.artist, formatDuration(track.durationMs.toLong())).joinToString(" · "), maxLines = 1)
        },
        leadingContent = { Cover(track.coverKey?.let(coverOf), Modifier.size(48.dp)) },
        trailingContent = { AddedMark(added) },
        modifier = Modifier.clickable(enabled = !added, onClick = onAdd),
    )
}

@Composable
private fun AddedMark(added: Boolean) {
    if (added) {
        Icon(Icons.Default.Check, stringResource(R.string.jam_added), tint = MaterialTheme.colorScheme.primary)
    } else {
        Icon(Icons.Default.Add, stringResource(R.string.jam_add_tracks))
    }
}

@Composable
private fun PhoneTab(
    index: LibraryIndex?,
    allowed: Boolean,
    onAllow: () -> Unit,
    onSend: (List<Track>) -> Unit,
) {
    if (!allowed) {
        Column(Modifier.fillMaxWidth().padding(32.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            Text(stringResource(R.string.jam_phone_access), textAlign = TextAlign.Center)
            Button(onClick = onAllow, modifier = Modifier.padding(top = 16.dp)) { Text(stringResource(R.string.access_allow)) }
        }
        return
    }
    if (index == null) return Loading()
    var query by rememberSaveable { mutableStateOf("") }
    val sent = remember { mutableStateListOf<Long>() }
    SearchField(query) { query = it }
    val tracks = remember(index, query) { index.search(query).tracks }
    if (tracks.isEmpty()) {
        Message(stringResource(if (query.isBlank()) R.string.empty_library else R.string.no_results))
        return
    }
    Box {
        LazyColumn {
            items(tracks, key = { it.id }) { t ->
                val done = t.id in sent
                ListItem(
                    headlineContent = { Text(t.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                    supportingContent = {
                        Text(
                            listOfNotNull(t.artist, formatDuration(t.durationMs)).joinToString(" · "),
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    },
                    leadingContent = { Cover(t.albumId, Modifier.size(48.dp)) },
                    trailingContent = { AddedMark(done) },
                    modifier = Modifier.clickable(enabled = !done) {
                        sent += t.id
                        onSend(listOf(t))
                    },
                )
            }
        }
    }
}
