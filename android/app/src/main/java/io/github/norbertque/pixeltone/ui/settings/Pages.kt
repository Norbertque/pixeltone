package io.github.norbertque.pixeltone.ui.settings

import android.content.Intent
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.OpenInNew
import androidx.compose.material.icons.filled.Computer
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.core.net.toUri
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.norbertque.pixeltone.BuildConfig
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.remote.PairedPc
import kotlinx.coroutines.launch

@Composable
fun PlaybackScreen(app: PixeltoneApp, onBack: () -> Unit) {
    val settings = app.playbackSettings
    val unplug by settings.pauseOnUnplug.collectAsStateWithLifecycle()
    val swiped by settings.stopWhenSwipedAway.collectAsStateWithLifecycle()
    PlaybackContent(unplug, settings::setPauseOnUnplug, swiped, settings::setStopWhenSwipedAway, onBack)
}

@Composable
fun PlaybackContent(
    pauseOnUnplug: Boolean,
    onPauseOnUnplug: (Boolean) -> Unit,
    stopWhenSwipedAway: Boolean,
    onStopWhenSwipedAway: (Boolean) -> Unit,
    onBack: () -> Unit,
) {
    SettingsScaffold(stringResource(R.string.playback), onBack) {
        item { SwitchRow(stringResource(R.string.pause_on_unplug), stringResource(R.string.pause_on_unplug_text), pauseOnUnplug, onPauseOnUnplug) }
        item {
            SwitchRow(stringResource(R.string.stop_when_swiped), stringResource(R.string.stop_when_swiped_text), stopWhenSwipedAway, onStopWhenSwipedAway)
        }
    }
}

/** A folder of the library and how many of its files are music. */
data class LibraryFolder(val path: String, val tracks: Int)

/** The library's folders, by path. */
fun libraryFolders(tracks: List<Track>): List<LibraryFolder> =
    tracks.groupingBy { it.folder }.eachCount().map { (path, count) -> LibraryFolder(path, count) }.sortedBy { it.path.lowercase() }

@Composable
fun LibrarySettingsScreen(app: PixeltoneApp, onBack: () -> Unit) {
    val settings = app.librarySettings
    val all by app.library.allTracks.collectAsStateWithLifecycle()
    val shortest by settings.shortestMs.collectAsStateWithLifecycle()
    val hidden by settings.hiddenFolders.collectAsStateWithLifecycle()
    val folders = remember(all) { libraryFolders(all.orEmpty()) }
    LibrarySettingsContent(shortest, settings::setShortest, folders, hidden, settings::setHidden, onRefresh = app.library::refresh, onBack = onBack)
}

@Composable
fun LibrarySettingsContent(
    shortestMs: Long,
    onShortest: (Long) -> Unit,
    folders: List<LibraryFolder>,
    hidden: Set<String>,
    onHidden: (String, Boolean) -> Unit,
    onRefresh: () -> Unit,
    onBack: () -> Unit,
) {
    var refreshed by remember { mutableStateOf(false) }
    SettingsScaffold(stringResource(R.string.library), onBack) {
        item {
            ChoiceRow(
                stringResource(R.string.leave_out_short),
                stringResource(R.string.leave_out_short_text),
                listOf(0L to stringResource(R.string.off)) + listOf(15, 30, 60).map { it * 1_000L to stringResource(R.string.seconds_short, it) },
                shortestMs,
                onShortest,
            )
        }
        item {
            ListItem(
                headlineContent = { Text(stringResource(R.string.read_library_again)) },
                supportingContent = { Text(stringResource(if (refreshed) R.string.read_library_again_done else R.string.read_library_again_text)) },
                modifier = Modifier.clickable {
                    onRefresh()
                    refreshed = true
                },
            )
        }
        if (folders.isNotEmpty()) {
            item { SectionTitle(stringResource(R.string.folders)) }
            item {
                Text(
                    stringResource(R.string.folders_text),
                    Modifier.padding(horizontal = 16.dp),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            items(folders, key = LibraryFolder::path) { folder ->
                SwitchRow(
                    folder.path.trimEnd('/').ifEmpty { "/" },
                    pluralStringResource(R.plurals.tracks_count, folder.tracks, folder.tracks),
                    checked = folder.path !in hidden,
                ) { shown -> onHidden(folder.path, !shown) }
            }
        }
    }
}

@Composable
fun LyricsSettingsScreen(app: PixeltoneApp, onBack: () -> Unit) {
    val online by app.lyrics.online.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    var forgotten by remember { mutableStateOf(false) }
    SettingsScaffold(stringResource(R.string.lyrics), onBack) {
        item { SwitchRow(stringResource(R.string.lyrics_online), stringResource(R.string.lyrics_online_text), online, app.lyrics::setOnline) }
        item {
            ListItem(
                headlineContent = { Text(stringResource(R.string.forget_lyrics)) },
                supportingContent = { Text(stringResource(if (forgotten) R.string.forget_lyrics_done else R.string.forget_lyrics_text)) },
                modifier = Modifier.clickable {
                    scope.launch {
                        app.lyrics.forget()
                        forgotten = true
                    }
                },
            )
        }
    }
}

@Composable
fun PcsScreen(app: PixeltoneApp, onBack: () -> Unit) {
    val pcs by app.remote.paired.pcs.collectAsStateWithLifecycle()
    PcsContent(pcs, onForget = { app.remote.paired.forget(it.id) }, onBack = onBack)
}

/** The PCs this phone paired with; each can be forgotten. */
@Composable
fun PcsContent(pcs: List<PairedPc>, onForget: (PairedPc) -> Unit, onBack: () -> Unit) {
    var forgetting by remember { mutableStateOf<PairedPc?>(null) }
    SettingsScaffold(stringResource(R.string.your_pcs), onBack) {
        if (pcs.isEmpty()) {
            item {
                Text(
                    stringResource(R.string.no_pcs),
                    Modifier.padding(16.dp),
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        items(pcs, key = PairedPc::id) { pc ->
            ListItem(
                headlineContent = { Text(pc.name) },
                supportingContent = { Text("${pc.host}:${pc.port}") },
                leadingContent = { Icon(Icons.Default.Computer, null) },
                trailingContent = { TextButton(onClick = { forgetting = pc }) { Text(stringResource(R.string.forget)) } },
            )
        }
    }
    forgetting?.let { pc ->
        AlertDialog(
            onDismissRequest = { forgetting = null },
            title = { Text(stringResource(R.string.forget_pc, pc.name)) },
            text = { Text(stringResource(R.string.forget_pc_text)) },
            confirmButton = {
                TextButton(onClick = {
                    onForget(pc)
                    forgetting = null
                }) { Text(stringResource(R.string.forget)) }
            },
            dismissButton = { TextButton(onClick = { forgetting = null }) { Text(stringResource(R.string.cancel)) } },
        )
    }
}

/** What looking for a newer build found. */
private enum class Looked { Looking, Newer, Newest, Offline }

@Composable
fun UpdatesScreen(app: PixeltoneApp, onBack: () -> Unit) {
    val updater = app.updater
    val automatic by updater.automatic.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    var looked by remember { mutableStateOf<Looked?>(null) }
    SettingsScaffold(stringResource(R.string.updates), onBack) {
        item {
            ListItem(
                headlineContent = { Text(stringResource(R.string.this_build, BuildConfig.VERSION_NAME)) },
                supportingContent = { Text(stringResource(R.string.this_build_text)) },
            )
        }
        item { SwitchRow(stringResource(R.string.look_for_builds), stringResource(R.string.look_for_builds_text), automatic, updater::setAutomatic) }
        item {
            ListItem(
                headlineContent = { Text(stringResource(R.string.look_now)) },
                supportingContent = {
                    Text(
                        stringResource(
                            when (looked) {
                                null -> R.string.look_now_text
                                Looked.Looking -> R.string.looking
                                Looked.Newer -> R.string.newer_build
                                Looked.Newest -> R.string.newest_build
                                Looked.Offline -> R.string.feed_offline
                            },
                        ),
                    )
                },
                modifier = Modifier.clickable(enabled = looked != Looked.Looking) {
                    looked = Looked.Looking
                    scope.launch {
                        looked = when (updater.checkNow()) {
                            true -> Looked.Newer
                            false -> Looked.Newest
                            null -> Looked.Offline
                        }
                    }
                },
            )
        }
    }
}

/** Where Pixeltone's source is. */
private const val SOURCE_URL = "https://github.com/Norbertque/pixeltone"

/** The work Pixeltone for Android is built on, and its licences. */
private val MADE_WITH = listOf(
    "Jetpack Compose, AndroidX, Media3" to "Apache 2.0",
    "Kotlin, kotlinx.coroutines, kotlinx.serialization" to "Apache 2.0",
    "Coil, OkHttp" to "Apache 2.0",
    "MaterialKolor (Material colour utilities)" to "Apache 2.0",
    "ZXing" to "Apache 2.0",
    "libusb" to "LGPL 2.1",
    "Google Sans Flex" to "SIL Open Font License 1.1",
)

@Composable
fun AboutScreen(onBack: () -> Unit) {
    val context = LocalContext.current
    SettingsScaffold(stringResource(R.string.about), onBack) {
        item {
            ListItem(
                headlineContent = { Text(stringResource(R.string.app_name), style = MaterialTheme.typography.titleLarge) },
                supportingContent = { Text(stringResource(R.string.about_version, BuildConfig.VERSION_NAME, BuildConfig.VERSION_CODE)) },
            )
        }
        item {
            ListItem(
                headlineContent = { Text(stringResource(R.string.source_code)) },
                supportingContent = { Text(SOURCE_URL.removePrefix("https://")) },
                trailingContent = { Icon(Icons.AutoMirrored.Filled.OpenInNew, null) },
                modifier = Modifier.clickable { context.startActivity(Intent(Intent.ACTION_VIEW, SOURCE_URL.toUri())) },
            )
        }
        item { SectionTitle(stringResource(R.string.made_with)) }
        items(MADE_WITH) { (what, licence) ->
            ListItem(headlineContent = { Text(what) }, supportingContent = { Text(licence) })
        }
        item {
            Text(
                stringResource(R.string.usb_licence),
                Modifier.padding(16.dp),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
