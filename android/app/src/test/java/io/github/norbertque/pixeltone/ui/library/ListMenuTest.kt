package io.github.norbertque.pixeltone.ui.library

import androidx.compose.foundation.layout.width
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.longClick
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.unit.dp
import com.github.takahirom.roborazzi.RobolectricDeviceQualifiers
import com.google.common.truth.Truth.assertThat
import io.github.norbertque.pixeltone.library.LibraryIndex
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.ui.theme.PixeltoneTheme
import java.util.Locale
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = RobolectricDeviceQualifiers.Pixel7, sdk = [35])
class ListMenuTest {
    @get:Rule
    val compose = createComposeRule()

    private val album = LibraryIndex(
        listOf(
            Track(1, "One", "Somebody", null, "Record", 1, 200_000, null, 1, null, "Music/R/", "1.flac"),
            Track(2, "Two", "Somebody", null, "Record", 1, 180_000, null, 2, null, "Music/R/", "2.flac"),
        ),
        Locale.ENGLISH,
    ).albums.single()

    @Test
    fun aLongPressOnAnAlbumCardOpensItsMenu() {
        var opened: TrackList? = null
        compose.setContent {
            PixeltoneTheme {
                CompositionLocalProvider(LocalListMenu provides { opened = it }) {
                    AlbumCard(album, onClick = {}, Modifier.width(160.dp))
                }
            }
        }
        compose.onNodeWithText("Record").performTouchInput { longClick() }
        assertThat(opened).isEqualTo(TrackList("Record", album.tracks, withPlay = true, albumKey = album.key))
    }

    @Test
    fun theAlbumPageMenuHasTheWholeAlbum() {
        var opened: TrackList? = null
        compose.setContent {
            PixeltoneTheme {
                CompositionLocalProvider(LocalListMenu provides { opened = it }) {
                    AlbumContent(album, loading = false, onBack = {}, onArtist = null, onPlay = {}, onShuffle = {})
                }
            }
        }
        compose.onNodeWithContentDescription("More").performClick()
        assertThat(opened).isEqualTo(TrackList("Record", album.tracks))
    }
}
