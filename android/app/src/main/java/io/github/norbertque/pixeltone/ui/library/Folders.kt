package io.github.norbertque.pixeltone.ui.library

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Folder
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.FolderEntry
import io.github.norbertque.pixeltone.library.FolderListing
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.library.fold
import io.github.norbertque.pixeltone.ui.BackButton
import io.github.norbertque.pixeltone.ui.Loading
import io.github.norbertque.pixeltone.ui.Message

/**
 * The library's Folders tab, as on Windows: the folders where the music starts. A search lists
 * every folder whose path has its words.
 */
@Composable
fun FoldersTab(top: FolderListing, everywhere: () -> List<FolderEntry>, query: String, onFolder: (String) -> Unit, onPlay: (List<Track>, Int) -> Unit) {
    val words = fold(query).split(' ').filter(String::isNotEmpty)
    if (words.isEmpty()) {
        FolderContent(top, onFolder, onPlay)
        return
    }
    val found = everywhere().filter { entry -> words.all { it in fold(entry.path) } }
    if (found.isEmpty()) {
        Message(stringResource(R.string.no_results, query.trim()))
        return
    }
    LazyColumn(Modifier.fillMaxSize()) {
        items(found, key = FolderEntry::path) { entry -> FolderRow(entry, showPath = true) { onFolder(entry.path) } }
    }
}

@Composable
fun FolderScreen(path: String, onBack: () -> Unit, onFolder: (String) -> Unit, onPlay: (List<Track>, Int) -> Unit, vm: LibraryViewModel = viewModel()) {
    val index by vm.index.collectAsStateWithLifecycle()
    val library = index
    FolderPage(library?.let { it.folder(path) to it.rootFolder }, onBack, onFolder, onPlay)
}

/** A folder on a page of its own: where it is (each part opens), its folders, its tracks. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FolderPage(folder: Pair<FolderListing, String>?, onBack: () -> Unit, onFolder: (String) -> Unit, onPlay: (List<Track>, Int) -> Unit) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(folder?.first?.name.orEmpty(), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = { BackButton(onBack) },
                actions = {
                    val listing = folder?.first
                    if (listing != null && listing.tracks.isNotEmpty()) ListMenuButton(TrackList(listing.name, listing.tracks, withPlay = true))
                },
            )
        },
    ) { padding ->
        when {
            folder == null -> Loading(Modifier.padding(padding))
            else -> Box(Modifier.padding(padding)) {
                val (listing, root) = folder
                FolderContent(listing, onFolder, onPlay, crumbs = { Crumbs(listing.path, root, onFolder) })
            }
        }
    }
}

/** The path from where the folders start, each part a button. */
@Composable
private fun Crumbs(path: String, root: String, onFolder: (String) -> Unit) {
    val parts = path.removePrefix(root).trim('/').split('/').filter(String::isNotEmpty)
    Row(
        Modifier.horizontalScroll(rememberScrollState()).padding(horizontal = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TextButton(onClick = { onFolder(root) }) { Text(stringResource(R.string.tab_folders)) }
        parts.forEachIndexed { i, part ->
            Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null, Modifier.size(18.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
            val at = (if (root.isEmpty()) "" else "$root/") + parts.take(i + 1).joinToString("/")
            if (i == parts.lastIndex) {
                Text(part, Modifier.padding(horizontal = 12.dp), style = MaterialTheme.typography.labelLarge)
            } else {
                TextButton(onClick = { onFolder(at) }) { Text(part) }
            }
        }
    }
}

@Composable
private fun FolderContent(
    listing: FolderListing,
    onFolder: (String) -> Unit,
    onPlay: (List<Track>, Int) -> Unit,
    crumbs: (@Composable () -> Unit)? = null,
) {
    if (listing.folders.isEmpty() && listing.tracks.isEmpty()) {
        Message(stringResource(R.string.empty_library))
        return
    }
    LazyColumn(Modifier.fillMaxSize()) {
        if (crumbs != null) item { crumbs() }
        items(listing.folders, key = { "f:" + it.path }) { entry -> FolderRow(entry, showPath = false) { onFolder(entry.path) } }
        itemsIndexed(listing.tracks, key = { _, track -> track.id }) { i, track ->
            TrackRow(track, showAlbum = false, onClick = { onPlay(listing.tracks, i) })
        }
    }
}

@Composable
private fun FolderRow(entry: FolderEntry, showPath: Boolean, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(entry.name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = {
            Text(
                listOfNotNull(
                    entry.path.substringBeforeLast('/', "").takeIf { showPath && it.isNotEmpty() },
                    pluralStringResource(R.plurals.tracks_count, entry.tracks, entry.tracks),
                ).joinToString(" · "),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        },
        leadingContent = {
            Box(
                Modifier.size(48.dp).clip(MaterialTheme.shapes.medium).background(MaterialTheme.colorScheme.secondaryContainer),
                contentAlignment = Alignment.Center,
            ) {
                Icon(Icons.Default.Folder, null, tint = MaterialTheme.colorScheme.onSecondaryContainer)
            }
        },
        trailingContent = { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null) },
        modifier = Modifier.clickable(onClick = onClick),
    )
}
