package io.github.norbertque.pixeltone.ui

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
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
import androidx.compose.runtime.CompositionLocalProvider
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
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.library.albumKey
import io.github.norbertque.pixeltone.ui.library.AddToPlaylistDialog
import io.github.norbertque.pixeltone.ui.library.AlbumScreen
import io.github.norbertque.pixeltone.ui.library.LocalAddToPlaylist
import io.github.norbertque.pixeltone.ui.library.LocalTrackMenu
import io.github.norbertque.pixeltone.ui.library.NameDialog
import io.github.norbertque.pixeltone.ui.library.PlaylistScreen
import io.github.norbertque.pixeltone.ui.library.TrackAction
import io.github.norbertque.pixeltone.ui.library.TrackMenu
import io.github.norbertque.pixeltone.ui.library.ArtistScreen
import io.github.norbertque.pixeltone.ui.library.LibraryScreen
import io.github.norbertque.pixeltone.ui.player.MiniPlayer
import io.github.norbertque.pixeltone.ui.player.NowPlayingScreen
import io.github.norbertque.pixeltone.ui.player.SleepSheet
import io.github.norbertque.pixeltone.ui.remote.PlayOnSheet
import io.github.norbertque.pixeltone.ui.lyrics.LyricsBody
import io.github.norbertque.pixeltone.ui.lyrics.rememberLyricsNow
import io.github.norbertque.pixeltone.ui.settings.SettingsScreen
import io.github.norbertque.pixeltone.ui.home.HomeScreen
import io.github.norbertque.pixeltone.ui.search.SearchScreen
import io.github.norbertque.pixeltone.ui.theme.PixeltoneTheme
import io.github.norbertque.pixeltone.ui.theme.seedOf
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.LibraryMusic
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.core.graphics.drawable.toBitmap
import coil.imageLoader
import coil.request.ImageRequest
import coil.request.SuccessResult
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import io.github.norbertque.pixeltone.ui.stats.StatsScreen
import io.github.norbertque.pixeltone.usb.usbBadge
import kotlinx.coroutines.launch

private const val HOME = "home"
private const val SEARCH = "search"
private const val LIBRARY = "library"
private const val NOW_PLAYING = "now-playing"
private const val JAM = "jam"
private const val SETTINGS = "settings"
private const val FAVOURITES = "favourites"
private const val STATS = "stats"

@Composable
fun PixeltoneRoot() {
    val context = LocalContext.current
    val app = remember { context.applicationContext as PixeltoneApp }
    PixeltoneTheme(seed = rememberCoverSeed(app)) { Root() }
}

/** The seed colour of the cover that plays, as on Windows: the whole app takes its colours from it. */
@Composable
private fun rememberCoverSeed(app: PixeltoneApp): Int? {
    val context = LocalContext.current
    val playback by app.player.state.collectAsStateWithLifecycle()
    val cover = playback.current?.cover
    var seed by remember { mutableStateOf<Int?>(null) }
    LaunchedEffect(cover) {
        if (cover == null) {
            seed = null
            return@LaunchedEffect
        }
        val request = ImageRequest.Builder(context).data(cover).size(112).allowHardware(false).build()
        val picture = (context.imageLoader.execute(request) as? SuccessResult)?.drawable
        seed = picture?.let { withContext(Dispatchers.Default) { seedOf(it.toBitmap()) } }
    }
    return seed
}

@Composable
private fun Root() {
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
    val lists by app.lists.lists.collectAsStateWithLifecycle()
    val index by app.library.index.collectAsStateWithLifecycle()
    var choosing by rememberSaveable { mutableStateOf(false) }
    /** The track whose menu is open; tracks waiting for a playlist; a new playlist's tracks. */
    var menuFor by remember { mutableStateOf<Track?>(null) }
    var adding by remember { mutableStateOf<List<Track>?>(null) }
    var naming by remember { mutableStateOf<List<Track>?>(null) }
    var sleeping by remember { mutableStateOf(false) }
    val sleep by app.sleep.state.collectAsStateWithLifecycle()
    val messages = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    val resources = LocalResources.current
    LaunchedEffect(Unit) { remote.connection.errors.collect { messages.showSnackbar(it) } }
    val nav = rememberNavController()
    JamNews(onPc?.jam, messages, onOpen = { nav.navigate(JAM) { launchSingleTop = true } })
    val route = nav.currentBackStackEntryAsState().value?.destination?.route
    // Until Now Playing has slid away, the bars stay hidden under it.
    val shown by nav.visibleEntries.collectAsStateWithLifecycle()
    val bars = shown.none { it.destination.route in FULL_SCREEN }
    // The phone's own track that plays (a PC's tracks aren't the phone's to list).
    val playingTrack = playback.current?.trackId?.takeIf { onPc == null && it >= 0 }?.let { index?.track(it) }
    CompositionLocalProvider(LocalTrackMenu provides { menuFor = it }, LocalAddToPlaylist provides { adding = it }) {
        Scaffold(
            snackbarHost = { SnackbarHost(messages) },
            bottomBar = {
                AnimatedVisibility(bars, enter = slideInVertically { it } + fadeIn(), exit = ExitTransition.None) {
                    Column {
                        playback.current?.let { entry ->
                            MiniPlayer(
                                entry,
                                playback.isPlaying,
                                onToggle = player::toggle,
                                onOpen = { nav.navigate(NOW_PLAYING) { launchSingleTop = true } },
                                playingOn = onPc?.pc?.name,
                                onNext = player::next,
                                onPrevious = player::previous,
                            )
                        }
                        BottomNavigation(route) { target ->
                            nav.navigate(target) {
                                popUpTo(HOME) { saveState = true }
                                launchSingleTop = true
                                restoreState = true
                            }
                        }
                    }
                }
            },
            // The screens inside handle the system bars themselves; this one only adds the mini player.
            contentWindowInsets = WindowInsets(0),
        ) { padding ->
            NavHost(
                nav,
                startDestination = HOME,
                modifier = Modifier.padding(padding).consumeWindowInsets(padding),
                enterTransition = { fadeIn(tween(FADE_MS)) },
                // Now Playing slides up over the screen it was opened from, and back down to it.
                exitTransition = {
                    if (targetState.destination.route == NOW_PLAYING) ExitTransition.KeepUntilTransitionsFinished else fadeOut(tween(FADE_MS))
                },
                popEnterTransition = { if (initialState.destination.route == NOW_PLAYING) EnterTransition.None else fadeIn(tween(FADE_MS)) },
                popExitTransition = { fadeOut(tween(FADE_MS)) },
            ) {
                composable(HOME) {
                    HomeScreen(
                        app,
                        onAlbum = { nav.navigate("album/${Uri.encode(it)}") },
                        onArtist = { nav.navigate("artist/${Uri.encode(it)}") },
                        onAlbums = { nav.navigate(LIBRARY) },
                        onStats = { nav.navigate(STATS) },
                        onSettings = { nav.navigate(SETTINGS) },
                    )
                }
                composable(SEARCH) {
                    SearchScreen(
                        onAlbum = { nav.navigate("album/${Uri.encode(it)}") },
                        onArtist = { nav.navigate("artist/${Uri.encode(it)}") },
                        onPlay = player::play,
                    )
                }
                composable(LIBRARY) {
                    LibraryScreen(
                        onAlbum = { nav.navigate("album/${Uri.encode(it)}") },
                        onArtist = { nav.navigate("artist/${Uri.encode(it)}") },
                        onPlay = player::play,
                        onPlayOn = { choosing = true },
                        onSettings = { nav.navigate(SETTINGS) },
                        lists = lists,
                        onFavourites = { nav.navigate(FAVOURITES) },
                        onPlaylist = { nav.navigate("playlist/$it") },
                        onNewPlaylist = { naming = emptyList() },
                        onStats = { nav.navigate(STATS) },
                    )
                }
                composable(STATS) {
                    StatsScreen(app, onBack = { nav.popBackStack() })
                }
                composable(FAVOURITES) {
                    PlaylistScreen(app, id = null, onBack = { nav.popBackStack() })
                }
                composable("playlist/{id}") { entry ->
                    PlaylistScreen(app, id = entry.arguments?.getString("id")?.toLongOrNull() ?: -1L, onBack = { nav.popBackStack() })
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
                composable(
                    NOW_PLAYING,
                    enterTransition = { slideInVertically(tween(SLIDE_MS, easing = Decelerate)) { it } },
                    popExitTransition = { slideOutVertically(tween(SLIDE_MS, easing = Accelerate)) { it } },
                ) {
                    NowPlayingScreen(
                        player,
                        onBack = { nav.popBackStack() },
                        playingOn = onPc?.pc?.name,
                        outputBadge = if (onPc != null) onPc?.state?.output?.badge?.label else usbBadge(usb),
                        onPlayOn = { choosing = true },
                        onJam = if (onPc != null) ({ nav.navigate(JAM) }) else null,
                        favourite = playingTrack?.let { lists.isFavourite(it) },
                        onFavourite = { playingTrack?.let { app.lists.setFavourite(it, !lists.isFavourite(it)) } },
                        onAddToPlaylist = playingTrack?.let { { adding = listOf(it) } },
                        sleep = sleep,
                        onSleep = { sleeping = true },
                        queueEditable = onPc == null,
                        lyrics = {
                            val now = rememberLyricsNow(app)
                            LyricsBody(now.state, now.online, now.positionMs, player::seekTo)
                        },
                    )
                }
                composable(JAM) {
                    OwnerJamScreen(app, onBack = { nav.popBackStack() })
                }
            }
        }
    }
    if (choosing) PlayOnSheet(remote, onDismiss = { choosing = false })
    if (sleeping) {
        SleepSheet(
            sleep = sleep,
            onMinutes = app.sleep::inMinutes,
            onEndOfTrack = app.sleep::atEndOfTrack,
            onCancel = app.sleep::cancel,
            onDismiss = { sleeping = false },
        )
    }
    menuFor?.let { track ->
        val album = albumKey(track)?.let { index?.album(it) }
        val artist = track.artist?.let { index?.artistNamed(it) }
        TrackMenu(
            track = track,
            favourite = lists.isFavourite(track),
            hasAlbum = album != null,
            hasArtist = artist != null,
            onAction = { action ->
                when (action) {
                    TrackAction.PlayNext -> player.playNext(listOf(track))
                    TrackAction.AddToQueue -> player.addToQueue(listOf(track))
                    TrackAction.AddToPlaylist -> adding = listOf(track)
                    TrackAction.Favourite -> app.lists.setFavourite(track, !lists.isFavourite(track))
                    TrackAction.Album -> album?.let { nav.navigate("album/${Uri.encode(it.key)}") }
                    TrackAction.Artist -> artist?.let { nav.navigate("artist/${Uri.encode(it.key)}") }
                }
            },
            onDismiss = { menuFor = null },
        )
    }
    adding?.let { tracks ->
        AddToPlaylistDialog(
            playlists = lists.playlists,
            count = tracks.size,
            onPick = { id ->
                adding = null
                app.lists.add(id, tracks)
                val name = lists.playlist(id)?.name.orEmpty()
                scope.launch { messages.showSnackbar(resources.getString(R.string.added_to, name)) }
            },
            onNew = {
                adding = null
                naming = tracks
            },
            onDismiss = { adding = null },
        )
    }
    naming?.let { tracks ->
        NameDialog(
            title = stringResource(R.string.new_playlist),
            initial = "",
            confirm = stringResource(R.string.create),
            onConfirm = { name ->
                naming = null
                val id = app.lists.create(name, tracks)
                if (tracks.isEmpty()) {
                    nav.navigate("playlist/$id")
                } else {
                    scope.launch { messages.showSnackbar(resources.getString(R.string.added_to, name)) }
                }
            },
            onDismiss = { naming = null },
        )
    }
}

/** Screens that take the whole height: no mini player or navigation under them. */
private val FULL_SCREEN = setOf(NOW_PLAYING, JAM)

private const val FADE_MS = 300
private const val SLIDE_MS = 350

/** Material's emphasized easing: in fast and settle, or set off slowly and leave fast. */
private val Decelerate = CubicBezierEasing(0.05f, 0.7f, 0.1f, 1f)
private val Accelerate = CubicBezierEasing(0.3f, 0f, 0.8f, 0.15f)

/** Home, Search and Library, as on Windows. */
@Composable
private fun BottomNavigation(route: String?, onGo: (String) -> Unit) {
    NavigationBar(containerColor = MaterialTheme.colorScheme.surfaceContainer) {
        listOf(
            Triple(HOME, Icons.Default.Home, R.string.home),
            Triple(SEARCH, Icons.Default.Search, R.string.search),
            Triple(LIBRARY, Icons.Default.LibraryMusic, R.string.library),
        ).forEach { (target, icon, label) ->
            NavigationBarItem(
                selected = route == target,
                onClick = { onGo(target) },
                icon = { Icon(icon, null) },
                label = { Text(stringResource(label)) },
            )
        }
    }
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
