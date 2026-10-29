package io.github.norbertque.pixeltone.ui

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.core.app.ActivityCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import io.github.norbertque.pixeltone.BuildConfig
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.AUDIO_PERMISSION
import io.github.norbertque.pixeltone.library.mayReadAudio
import io.github.norbertque.pixeltone.remote.PcJam
import io.github.norbertque.pixeltone.ui.jam.GuestJamScreen
import io.github.norbertque.pixeltone.ui.jam.OwnerJamScreen
import io.github.norbertque.pixeltone.ui.library.AlbumScreen
import io.github.norbertque.pixeltone.ui.library.ArtistScreen
import io.github.norbertque.pixeltone.ui.library.LibraryScreen
import io.github.norbertque.pixeltone.ui.player.MiniPlayer
import io.github.norbertque.pixeltone.ui.player.NowPlayingScreen
import io.github.norbertque.pixeltone.ui.player.QueueScreen
import io.github.norbertque.pixeltone.ui.remote.PlayOnSheet
import io.github.norbertque.pixeltone.ui.settings.SettingsScreen
import io.github.norbertque.pixeltone.usb.usbBadge
import kotlinx.coroutines.launch

private const val LIBRARY = "library"
private const val NOW_PLAYING = "now-playing"
private const val QUEUE = "queue"
private const val JAM = "jam"
private const val SETTINGS = "settings"

@Composable
fun PixeltoneRoot() {
    val context = LocalContext.current
    var allowed by remember { mutableStateOf(context.mayReadAudio()) }
    // Access can be given in the system settings while the app waits in the background.
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { allowed = context.mayReadAudio() }
    val app = remember { context.applicationContext as PixeltoneApp }
    val guest by app.jam.guest.collectAsStateWithLifecycle()
    Box {
        if (allowed) {
            LaunchedEffect(Unit) {
                app.library.start()
                app.player.connect()
            }
            Navigation()
        } else {
            AccessScreen(onAllowed = { allowed = true })
        }
        // Someone else's jam, over everything: joining needs no access to this phone's music.
        if (guest != null) GuestJamScreen(app)
    }
    val updater = remember { (context.applicationContext as PixeltoneApp).updater }
    val update by updater.state.collectAsStateWithLifecycle()
    LaunchedEffect(Unit) { updater.checkOnce() }
    UpdateDialog(update, BuildConfig.VERSION_NAME, onUpdate = updater::update, onDismiss = updater::dismiss)
}

@Composable
private fun Navigation() {
    val context = LocalContext.current
    val app = remember { context.applicationContext as PixeltoneApp }
    val player = app.player
    val remote = app.remote
    val playback by player.state.collectAsStateWithLifecycle()
    val onPc by remote.connection.session.collectAsStateWithLifecycle()
    val usb by app.usb.state.collectAsStateWithLifecycle()
    var choosing by rememberSaveable { mutableStateOf(false) }
    val messages = remember { SnackbarHostState() }
    LaunchedEffect(Unit) { remote.connection.errors.collect { messages.showSnackbar(it) } }
    val nav = rememberNavController()
    JamNews(onPc?.jam, messages, onOpen = { nav.navigate(JAM) { launchSingleTop = true } })
    val route = nav.currentBackStackEntryAsState().value?.destination?.route
    Scaffold(
        snackbarHost = { SnackbarHost(messages) },
        bottomBar = {
            val entry = playback.current
            if (entry != null && route != NOW_PLAYING && route != QUEUE && route != JAM) {
                MiniPlayer(
                    entry,
                    playback.isPlaying,
                    onToggle = player::toggle,
                    onOpen = { nav.navigate(NOW_PLAYING) },
                    playingOn = onPc?.pc?.name,
                )
            }
        },
        // The screens inside handle the system bars themselves; this one only adds the mini player.
        contentWindowInsets = WindowInsets(0),
    ) { padding ->
        NavHost(nav, startDestination = LIBRARY, modifier = Modifier.padding(padding).consumeWindowInsets(padding)) {
            composable(LIBRARY) {
                LibraryScreen(
                    onAlbum = { nav.navigate("album/${Uri.encode(it)}") },
                    onArtist = { nav.navigate("artist/${Uri.encode(it)}") },
                    onPlay = player::play,
                    onPlayOn = { choosing = true },
                    onSettings = { nav.navigate(SETTINGS) },
                )
            }
            composable(SETTINGS) {
                SettingsScreen(app, onBack = { nav.popBackStack() })
            }
            composable("album/{key}") { entry ->
                AlbumScreen(
                    key = entry.arguments?.getString("key").orEmpty(),
                    onBack = { nav.popBackStack() },
                    onArtist = { nav.navigate("artist/${Uri.encode(it)}") },
                    onPlay = player::play,
                    onShuffle = player::shuffle,
                )
            }
            composable("artist/{key}") { entry ->
                ArtistScreen(
                    key = entry.arguments?.getString("key").orEmpty(),
                    onBack = { nav.popBackStack() },
                    onAlbum = { nav.navigate("album/${Uri.encode(it)}") },
                    onPlay = player::play,
                )
            }
            composable(NOW_PLAYING) {
                NowPlayingScreen(
                    player,
                    onBack = { nav.popBackStack() },
                    onQueue = { nav.navigate(QUEUE) },
                    playingOn = onPc?.pc?.name,
                    outputBadge = if (onPc != null) onPc?.state?.output?.badge?.label else usbBadge(usb),
                    onPlayOn = { choosing = true },
                    onJam = if (onPc != null) ({ nav.navigate(JAM) }) else null,
                )
            }
            composable(JAM) {
                OwnerJamScreen(app, onBack = { nav.popBackStack() })
            }
            composable(QUEUE) {
                QueueScreen(player, onBack = { nav.popBackStack() }, editable = onPc == null)
            }
        }
    }
    if (choosing) PlayOnSheet(remote, onDismiss = { choosing = false })
}

/** The owner hears about the jam wherever they are in the app: who came and went, who asks to come back. */
@Composable
private fun JamNews(jam: PcJam?, messages: SnackbarHostState, onOpen: () -> Unit) {
    val resources = LocalResources.current
    val scope = rememberCoroutineScope()
    var last by remember { mutableStateOf<PcJam?>(null) }
    LaunchedEffect(jam) {
        val before = last
        last = jam
        if (before == null || jam == null) return@LaunchedEffect
        for (g in jam.guests.filter { g -> before.guests.none { it.id == g.id } }) {
            scope.launch { messages.showSnackbar(resources.getString(R.string.jam_joined, g.name)) }
        }
        for (g in before.guests.filter { g -> jam.guests.none { it.id == g.id } }) {
            scope.launch { messages.showSnackbar(resources.getString(R.string.jam_left, g.name)) }
        }
        for (r in jam.requests.filter { r -> before.requests.none { it.id == r.id } }) {
            scope.launch {
                val answer = messages.showSnackbar(
                    resources.getString(R.string.jam_request, r.name),
                    actionLabel = resources.getString(R.string.jam_open),
                    duration = SnackbarDuration.Long,
                )
                if (answer == SnackbarResult.ActionPerformed) onOpen()
            }
        }
    }
}

@Composable
internal fun AccessScreen(onAllowed: () -> Unit) {
    val context = LocalContext.current
    var refused by rememberSaveable { mutableStateOf(false) }
    val request = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        if (granted) onAllowed() else refused = true
    }
    // Once Android stops asking ("don't ask again"), only the app's settings can change it.
    val askable = !refused ||
        context.findActivity()?.let { ActivityCompat.shouldShowRequestPermissionRationale(it, AUDIO_PERMISSION) } == true
    Surface(Modifier.fillMaxSize()) {
        Column(
            Modifier.fillMaxSize().safeDrawingPadding().padding(32.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            PixeltoneGlyph(Modifier.size(56.dp))
            Spacer(Modifier.height(24.dp))
            Text(stringResource(R.string.access_title), style = MaterialTheme.typography.headlineSmall)
            Spacer(Modifier.height(12.dp))
            Text(
                stringResource(if (askable) R.string.access_text else R.string.access_denied),
                textAlign = TextAlign.Center,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.height(24.dp))
            if (askable) {
                Button(onClick = { request.launch(AUDIO_PERMISSION) }) { Text(stringResource(R.string.access_allow)) }
            } else {
                Button(onClick = { context.openAppSettings() }) { Text(stringResource(R.string.access_settings)) }
            }
        }
    }
}

private fun Context.openAppSettings() {
    startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", packageName, null)))
}

private tailrec fun Context.findActivity(): Activity? =
    when (this) {
        is Activity -> this
        is ContextWrapper -> baseContext.findActivity()
        else -> null
    }
