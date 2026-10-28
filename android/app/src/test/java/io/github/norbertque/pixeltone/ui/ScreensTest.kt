package io.github.norbertque.pixeltone.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import androidx.media3.common.Player
import com.github.takahirom.roborazzi.RobolectricDeviceQualifiers
import com.github.takahirom.roborazzi.captureRoboImage
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
import io.github.norbertque.pixeltone.ui.theme.PixeltoneTheme
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

    private fun shot(name: String, dark: Boolean = false, content: @Composable () -> Unit) {
        compose.setContent { PixeltoneTheme(dark = dark, content = content) }
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
    fun queue() = shot("queue") { QueueContent(playing, NoControls, onBack = {}) }

    @Test
    fun miniPlayer() = shot("mini-player") {
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.BottomCenter) {
            MiniPlayer(playing.current!!, isPlaying = true, onToggle = {}, onOpen = {})
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

private fun entry(track: Track) = QueueEntry(track.id, track.title, track.artist, track.album, track.albumId)

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
