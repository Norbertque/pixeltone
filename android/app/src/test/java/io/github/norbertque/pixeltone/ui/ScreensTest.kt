package io.github.norbertque.pixeltone.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.media3.common.Player
import com.github.takahirom.roborazzi.RobolectricDeviceQualifiers
import com.github.takahirom.roborazzi.captureRoboImage
import io.github.norbertque.pixeltone.library.AlbumCover
import io.github.norbertque.pixeltone.library.LibraryIndex
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.playback.PlaybackState
import io.github.norbertque.pixeltone.playback.PlayerControls
import io.github.norbertque.pixeltone.playback.QueueEntry
import io.github.norbertque.pixeltone.ui.library.AlbumContent
import io.github.norbertque.pixeltone.ui.library.ArtistContent
import io.github.norbertque.pixeltone.ui.library.LibraryContent
import io.github.norbertque.pixeltone.ui.player.MiniPlayer
import io.github.norbertque.pixeltone.ui.player.NowPlayingContent
import io.github.norbertque.pixeltone.ui.player.QueueContent
import io.github.norbertque.pixeltone.remote.FoundPc
import io.github.norbertque.pixeltone.jam.Uploads
import io.github.norbertque.pixeltone.remote.GuestPermissions
import io.github.norbertque.pixeltone.remote.GuestView
import io.github.norbertque.pixeltone.remote.JamGuest
import io.github.norbertque.pixeltone.remote.PairedPc
import io.github.norbertque.pixeltone.remote.PcJam
import io.github.norbertque.pixeltone.remote.PcQueueRow
import io.github.norbertque.pixeltone.remote.PcState
import io.github.norbertque.pixeltone.remote.PcTrack
import io.github.norbertque.pixeltone.remote.RejoinRequest
import io.github.norbertque.pixeltone.ui.jam.JamContent
import io.github.norbertque.pixeltone.ui.jam.JoinContent
import io.github.norbertque.pixeltone.ui.jam.OwnerPart
import io.github.norbertque.pixeltone.ui.jam.StartJamContent
import io.github.norbertque.pixeltone.ui.jam.WaitingContent
import io.github.norbertque.pixeltone.library.ListedTrack
import io.github.norbertque.pixeltone.library.Lists
import io.github.norbertque.pixeltone.library.Playlist
import io.github.norbertque.pixeltone.ui.library.ListRow
import io.github.norbertque.pixeltone.ui.library.PlaylistContent
import io.github.norbertque.pixeltone.library.Listen
import io.github.norbertque.pixeltone.library.listeningStats
import io.github.norbertque.pixeltone.ui.stats.StatsContent
import io.github.norbertque.pixeltone.lyrics.Lyrics
import io.github.norbertque.pixeltone.lyrics.parseLrc
import io.github.norbertque.pixeltone.ui.lyrics.LyricsContent
import io.github.norbertque.pixeltone.ui.lyrics.LyricsState
import io.github.norbertque.pixeltone.widget.PlayerWidget
import io.github.norbertque.pixeltone.ui.home.HomeContent
import io.github.norbertque.pixeltone.ui.home.HomeData
import io.github.norbertque.pixeltone.ui.search.SearchContent
import io.github.norbertque.pixeltone.widget.WidgetState
import android.widget.FrameLayout
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.ui.unit.dp
import io.github.norbertque.pixeltone.ui.settings.SettingsContent
import io.github.norbertque.pixeltone.ui.settings.UsbPanel
import io.github.norbertque.pixeltone.usb.DAC
import io.github.norbertque.pixeltone.usb.SAMPLE_DAC
import io.github.norbertque.pixeltone.usb.UsbExclusiveState
import io.github.norbertque.pixeltone.usbaudio.negotiation.Conversion
import io.github.norbertque.pixeltone.usbaudio.negotiation.NegotiatedFormat
import io.github.norbertque.pixeltone.usbaudio.negotiation.SourceFormat
import io.github.norbertque.pixeltone.ui.remote.PlayOnContent
import io.github.norbertque.pixeltone.ui.remote.pcChoices
import androidx.compose.material3.Surface
import io.github.norbertque.pixeltone.ui.theme.PixeltoneTheme
import io.github.norbertque.pixeltone.update.UpdateInfo
import io.github.norbertque.pixeltone.update.UpdateState
import java.util.Locale
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/**
 * Renders the screens without a phone: `./gradlew recordRoborazziDebug` writes them to
 * `app/build/outputs/roborazzi/`. Look at them after changing a screen.
 */
@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
// Android 15: Espresso, under the Compose test rule, doesn't know Android 17's input manager yet.
@Config(qualifiers = RobolectricDeviceQualifiers.Pixel7, sdk = [35])
class ScreensTest {
    @get:Rule
    val compose = createComposeRule()

    private val index = LibraryIndex(SAMPLE, Locale.ENGLISH)

    private val playing = PlaybackState(
        queue = index.albums.first { it.title == "Rubikon" }.tracks.map(::entry) + index.albums.first { it.title == "1" }.tracks.map(::entry),
        index = 1,
        isPlaying = true,
        durationMs = 247_000,
        repeatMode = Player.REPEAT_MODE_ALL,
        shuffle = false,
    )

    // The clock runs by hand: some screens animate for as long as they show (the seek bar's wave).
    private fun shot(name: String, dark: Boolean = false, content: @Composable () -> Unit) {
        compose.mainClock.autoAdvance = false
        compose.setContent { PixeltoneTheme(dark = dark, content = content) }
        compose.mainClock.advanceTimeBy(1_000)
        compose.onRoot().captureRoboImage("build/outputs/roborazzi/$name.png")
    }

    @Composable
    private fun Library(tab: Int, query: String = "", searching: Boolean = false, library: LibraryIndex = index) {
        LibraryContent(
            results = library.search(query),
            query = query,
            tab = tab,
            searching = searching,
            onTab = {},
            onSearching = {},
            onQuery = {},
            onAlbum = {},
            onArtist = {},
            onPlay = { _, _ -> },
        )
    }

    @Test
    fun tracks() = shot("library-tracks") { Library(tab = 0) }

    @Test
    fun albumsDark() = shot("library-albums-dark", dark = true) { Library(tab = 1) }

    @Test
    fun artists() = shot("library-artists") { Library(tab = 2) }

    @Test
    fun search() = shot("library-search") { Library(tab = 0, query = "rubikon", searching = true) }

    @Test
    fun searchWithoutMatch() = shot("library-search-nothing", dark = true) { Library(tab = 1, query = "xyz", searching = true) }

    @Test
    fun emptyLibrary() = shot("library-empty") { Library(tab = 0, library = LibraryIndex(emptyList())) }

    @Test
    fun album() = shot("album") {
        AlbumContent(index.albums.first { it.title == "Rubikon" }, loading = false, onBack = {}, onArtist = {}, onPlay = {}, onShuffle = {})
    }

    @Test
    fun compilationDark() = shot("album-compilation-dark", dark = true) {
        AlbumContent(index.albums.first { it.isCompilation }, loading = false, onBack = {}, onArtist = null, onPlay = {}, onShuffle = {})
    }

    @Test
    fun artist() = shot("artist") {
        ArtistContent(index.artistNamed("Kryštof"), loading = false, onBack = {}, onAlbum = {}, onPlay = { _, _ -> })
    }

    @Test
    fun access() = shot("access") { AccessScreen(onAllowed = {}) }

    @Test
    fun nowPlaying() = shot("now-playing") { NowPlayingContent(playing, 95_000, NoControls, onBack = {}, onQueue = {}) }

    @Test
    fun nowPlayingDark() = shot("now-playing-dark", dark = true) {
        NowPlayingContent(playing.copy(isPlaying = false, shuffle = true, repeatMode = Player.REPEAT_MODE_ONE), 0, NoControls, {}, {})
    }

    @Test
    fun repeatHint() {
        compose.mainClock.autoAdvance = false
        compose.setContent { PixeltoneTheme { NowPlayingContent(playing, 95_000, NoControls, onBack = {}, onQueue = {}) } }
        compose.onNodeWithContentDescription("Repeat: all").performClick()
        compose.mainClock.advanceTimeBy(1_000)
        compose.onRoot().captureRoboImage("build/outputs/roborazzi/now-playing-repeat-hint.png")
    }

    @Test
    fun queue() = shot("queue") { QueueContent(playing, NoControls, onBack = {}) }

    @Test
    fun updateOffered() = shot("update-offered") {
        Library(tab = 1)
        UpdateDialog(UpdateState.Available(UPDATE), "0.1.9", onUpdate = {}, onDismiss = {})
    }

    @Test
    fun updateDownloading() = shot("update-downloading", dark = true) {
        Library(tab = 0)
        UpdateDialog(UpdateState.Downloading(UPDATE, 0.42f), "0.1.9", onUpdate = {}, onDismiss = {})
    }

    @Test
    fun playOn() = shot("play-on") {
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.BottomCenter) {
            Surface(color = androidx.compose.material3.MaterialTheme.colorScheme.surfaceContainerLow) {
                PlayOnContent(
                    choices = pcChoices(listOf(FOUND_BIG_ONE, FOUND_NEW), listOf(BIG_ONE, OLD_PC)),
                    activeId = BIG_ONE.id,
                    connected = true,
                    onPhone = {},
                    onPc = {},
                    onForget = {},
                    onAddress = {},
                )
            }
        }
    }

    @Test
    fun nowPlayingOnPc() = shot("now-playing-on-pc") {
        NowPlayingContent(playing, 95_000, NoControls, {}, {}, playingOn = "BIG-ONE", outputBadge = "Exclusive • 24-bit/96 kHz • Bit-perfect")
    }

    @Test
    fun queueOnPc() = shot("queue-on-pc") { QueueContent(playing, NoControls, onBack = {}, editable = false) }

    @Test
    fun jamJoin() = shot("jam-join") {
        JoinContent("BIG-ONE", "Petr", busy = false, error = null, onJoin = {}, onCancel = {})
    }

    @Test
    fun jamJoinFailedDark() = shot("jam-join-failed-dark", dark = true) {
        JoinContent("BIG-ONE", "Petr", busy = false, error = "Can't reach BIG-ONE. Is this phone on the same Wi-Fi?", onJoin = {}, onCancel = {})
    }

    @Test
    fun jamWaiting() = shot("jam-waiting") { WaitingContent(onCancel = {}) }

    @Test
    fun jamStart() = shot("jam-start") { StartJamContent("BIG-ONE", onStart = {}, onBack = {}) }

    @Test
    fun jamGuest() = shot("jam-guest") {
        JamContent(
            title = "BIG-ONE’s jam",
            connected = true,
            state = JAM_STATE,
            playing = JAM_ROWS.first(),
            upNext = JAM_ROWS.drop(1),
            me = GuestView("g-petr", "Petr", GuestPermissions()),
            owner = null,
            uploads = Uploads(total = 3, done = 1, current = "Night Swim", fraction = 0.42f),
            coverOf = { it },
            onBack = {},
            onFinish = {},
            onCommand = {},
            onAddTracks = {},
        )
    }

    @Test
    fun jamOwnerDark() = shot("jam-owner-dark", dark = true) {
        JamContent(
            title = "Jam",
            connected = true,
            state = JAM_STATE,
            playing = JAM_ROWS.first(),
            upNext = JAM_ROWS.drop(1),
            me = null,
            owner = OwnerPart(
                jam = PcJam(
                    link = "https://norbertque.github.io/pixeltone/jam/#v=1&h=192.168.1.20&p=47821&f=${"ab".repeat(32)}&s=secret&n=BIG-ONE",
                    guests = listOf(JamGuest("g-petr", "Petr"), JamGuest("g-jana", "Jana")),
                    requests = listOf(RejoinRequest("r1", "Tomáš")),
                ),
                onShare = {},
                onRemoveGuest = {},
                onPermissions = {},
                onAnswer = { _, _ -> },
            ),
            uploads = Uploads(),
            coverOf = { it },
            onBack = {},
            onFinish = {},
            onCommand = {},
            onAddTracks = {},
        )
    }

    @Test
    fun settingsOff() = shot("settings-usb-off") { Settings(UsbPanel(false, UsbExclusiveState.Disabled, null, null, false, false)) }

    @Test
    fun settingsPlaying() = shot("settings-usb-playing") {
        val active = UsbExclusiveState.Active(
            device = DAC,
            capabilities = SAMPLE_DAC,
            format = NegotiatedFormat(SAMPLE_DAC.formats[1], 96_000, Conversion.BIT_PERFECT),
            source = SourceFormat(96_000, 24, 2, isFloat = true),
            conversion = Conversion.BIT_PERFECT,
            hardwareVolume = false,
            softwareVolumeDb = -30f,
        )
        Settings(UsbPanel(true, active, -30f, null, false, false))
    }

    @Test
    fun settingsDeniedDark() = shot("settings-usb-denied-dark", dark = true) {
        Settings(UsbPanel(true, UsbExclusiveState.PermissionDenied(DAC.copy(hasPermission = false)), null, null, false, false))
    }

    @Test
    fun nowPlayingUsb() = shot("now-playing-usb") {
        NowPlayingContent(playing, 95_000, NoControls, {}, {}, outputBadge = "USB • 24-bit/96 kHz • Bit-perfect")
    }

    @Composable
    private fun Settings(usb: UsbPanel) =
        SettingsContent(usb, onBack = {}, onEnabled = {}, onRetry = {}, onHardwareVolume = {}, onSoftwareVolume = {}, onAcknowledgeLoudness = {}, onFixedVolume = {})

    @Test
    fun playlistsTab() = shot("library-playlists") {
        LibraryContent(
            results = index.search(""),
            query = "",
            tab = 3,
            searching = false,
            onTab = {},
            onSearching = {},
            onQuery = {},
            onAlbum = {},
            onArtist = {},
            onPlay = { _, _ -> },
            lists = Lists(
                favourites = index.tracks.take(12).map(ListedTrack::of),
                playlists = listOf(
                    Playlist(1, "Road trip", index.tracks.take(23).map(ListedTrack::of)),
                    Playlist(2, "Late night", index.tracks.drop(5).take(8).map(ListedTrack::of)),
                ),
            ),
        )
    }

    @Test
    fun playlist() = shot("playlist") {
        val rows = index.tracks.take(6).map { ListRow(ListedTrack.of(it), it) } +
            ListRow(ListedTrack("Music/Gone/x.flac", "A track that left", "Somebody"), null)
        PlaylistContent("Road trip", rows, reorderable = true, {}, {}, {}, {}, { _, _ -> }, {}, {})
    }

    @Test
    fun nowPlayingFavourite() = shot("now-playing-favourite", dark = true) {
        NowPlayingContent(playing, 95_000, NoControls, {}, {}, favourite = true, onAddToPlaylist = {})
    }

    @Test
    fun stats() = shot("stats") {
        val now = 100L * 86_400_000
        val listens = index.tracks.flatMapIndexed { i, track ->
            List(index.tracks.size - i) { n -> Listen(ListedTrack.of(track), now - (i * 7 + n) * 3_600_000L, track.durationMs) }
        }
        StatsContent(listeningStats(listens, since = null), period = 2, now = now, onPeriod = {}, onBack = {}, onTrack = {})
    }

    @Test
    fun lyricsSynced() = shot("lyrics-synced", dark = true) {
        val lrc = (0 until 24).joinToString("\n") { i -> "[%02d:%02d.00]Line number ${i + 1} of the song".format(i * 4 / 60, i * 4 % 60) }
        val (lines, _) = parseLrc(lrc)
        LyricsContent("Ty a já", "Kryštof", LyricsState.Found(Lyrics(lines, true, Lyrics.Source.LRCLIB)), true, 41_000, {}, {})
    }

    @Test
    fun lyricsMissing() = shot("lyrics-missing") {
        LyricsContent("Ty a já", "Kryštof", LyricsState.Missing, online = false, positionMs = 0, onSeek = {}, onBack = {})
    }

    @Test
    fun nowPlayingSleep() = shot("now-playing-sleep") {
        NowPlayingContent(playing, 95_000, NoControls, {}, {}, favourite = false, sleep = "Stops in 23:41")
    }

    @Test
    fun widget() = shot("widget") {
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            AndroidView(
                factory = { context ->
                    PlayerWidget.views(context, WidgetState("Ty a já", "Kryštof", playing = true), cover = null)
                        .apply(context, FrameLayout(context))
                },
                modifier = Modifier.fillMaxWidth().height(96.dp).padding(horizontal = 16.dp),
            )
        }
    }

    private val homeData by lazy {
        HomeData(
            tracks = 1863,
            albums = 170,
            artists = 60,
            durationMs = 509_000_000,
            mix = index.tracks.take(12),
            monthMs = 27_360_000,
            recentlyPlayed = index.albums.take(4),
            topArtists = index.artists.take(4).mapIndexed { i, a -> a to 24 - i * 3 },
            recentlyAdded = index.albums.takeLast(4),
        )
    }

    @Test
    fun home() = shot("home") { HomeContent(homeData, hour = 9, {}, {}, {}, {}, {}, {}, {}) }

    @Test
    fun homeDark() = shot("home-dark", dark = true) { HomeContent(homeData, hour = 20, {}, {}, {}, {}, {}, {}, {}) }

    @Test
    fun searchResults() = shot("search") { SearchContent("ry", index.search("ry"), {}, {}, {}, { _, _ -> }) }

    @Test
    fun miniPlayer() = shot("mini-player") {
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.BottomCenter) {
            MiniPlayer(playing.current!!, isPlaying = true, onToggle = {}, onOpen = {}, playingOn = "BIG-ONE")
        }
    }
}

private object NoControls : PlayerControls {
    override fun toggle() {}
    override fun next() {}
    override fun previous() {}
    override fun seekTo(positionMs: Long) {}
    override fun jumpTo(index: Int) {}
    override fun move(from: Int, to: Int) {}
    override fun remove(index: Int) {}
    override fun cycleRepeat() {}
    override fun toggleShuffle() {}
}

private val JAM_STATE = PcState(
    status = "playing",
    track = PcTrack(title = "Rubikon", artist = "Lenka Dusilová", durationMs = 241_000.0),
    queueIndex = 0,
    queueLength = 4,
)

private val JAM_ROWS = listOf(
    PcQueueRow(1, title = "Rubikon", artist = "Lenka Dusilová", durationMs = 241_000.0),
    PcQueueRow(2, title = "Night Swim", artist = "Hidden Orchestra", durationMs = 312_000.0, addedBy = "Petr", guestId = "g-petr"),
    PcQueueRow(3, title = "Tidal", artist = "Floating Points", durationMs = 284_000.0, addedBy = "Jana", guestId = "g-jana"),
    PcQueueRow(4, title = "Glass", artist = "Maribou State", durationMs = 263_000.0),
)

private val BIG_ONE = PairedPc("big", "BIG-ONE", "192.168.1.20", 47821, "ab".repeat(32), "token")
private val OLD_PC = PairedPc("old", "OLD-PC", "192.168.1.30", 47821, "cd".repeat(32), "token")
private val FOUND_BIG_ONE = FoundPc("BIG-ONE", "big", "BIG-ONE", "192.168.1.20", 47821)
private val FOUND_NEW = FoundPc("WORK-PC", "work", "WORK-PC", "192.168.1.40", 47821)

private val UPDATE = UpdateInfo(12, "0.1.12", "https://example.com/Pixeltone-android.apk", "a".repeat(64))

private fun entry(track: Track) = QueueEntry(track.id, track.title, track.artist, track.album, AlbumCover(track.albumId))

private var nextId = 1L

private fun track(title: String, artist: String?, album: String?, number: Int?, ms: Long, albumArtist: String? = null, year: Int? = null) =
    Track(nextId++, title, artist, albumArtist, album, nextId, ms, null, number, year, "Music/$album/")

private val SAMPLE = listOf(
    track("Inzerát", "Kryštof", "Rubikon", 1, 214_000, year = 2004),
    track("Ty a já", "Kryštof", "Rubikon", 2, 247_000, year = 2004),
    track("Rubikon", "Kryštof", "Rubikon", 3, 263_000, year = 2004),
    track("Cesta", "Kryštof", "Inzerát", 1, 236_000, year = 2003),
    track("Hey Jude", "The Beatles", "1", 7, 431_000, year = 2000),
    track("Let It Be", "The Beatles", "1", 25, 230_000, year = 2000),
    track("Bohemian Rhapsody", "Queen", "A Night at the Opera", 11, 355_000, year = 1975),
    track("A Very Long Title That Will Not Fit On One Line Of A Phone Screen", "Queen", "A Night at the Opera", 4, 189_000, year = 1975),
    track("Clair de lune", "Claude Debussy", "Piano Works", 3, 301_000),
    track("Summer Mix Opener", "DJ One", "Summer Mix", 1, 180_000),
    track("Summer Mix Closer", "DJ Two", "Summer Mix", 2, 205_000),
    track("Untagged recording", null, null, null, 61_000),
)
