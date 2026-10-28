package io.github.norbertque.pixeltone.ui

import androidx.compose.runtime.Composable
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.RobolectricDeviceQualifiers
import com.github.takahirom.roborazzi.captureRoboImage
import io.github.norbertque.pixeltone.library.LibraryIndex
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.ui.library.AlbumContent
import io.github.norbertque.pixeltone.ui.library.ArtistContent
import io.github.norbertque.pixeltone.ui.library.LibraryContent
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

    private fun shot(name: String, dark: Boolean = false, content: @Composable () -> Unit) {
        compose.setContent { PixeltoneTheme(dark = dark, content = content) }
        compose.onRoot().captureRoboImage("build/outputs/roborazzi/$name.png")
    }

    @Composable
    private fun Library(tab: Int, query: String = "", searching: Boolean = false, library: LibraryIndex = index) {
        LibraryContent(library.search(query), query, tab, searching, {}, {}, {}, {}, {})
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
        AlbumContent(index.albums.first { it.title == "Rubikon" }, loading = false, onBack = {}, onArtist = {})
    }

    @Test
    fun compilationDark() = shot("album-compilation-dark", dark = true) {
        AlbumContent(index.albums.first { it.isCompilation }, loading = false, onBack = {}, onArtist = null)
    }

    @Test
    fun artist() = shot("artist") { ArtistContent(index.artistNamed("Kryštof"), loading = false, onBack = {}, onAlbum = {}) }

    @Test
    fun access() = shot("access") { AccessScreen(onAllowed = {}) }
}

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
