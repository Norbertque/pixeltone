package io.github.norbertque.pixeltone.ui.player

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.swipe
import androidx.compose.ui.test.swipeDown
import androidx.compose.ui.test.swipeLeft
import androidx.compose.ui.test.swipeUp
import com.github.takahirom.roborazzi.RobolectricDeviceQualifiers
import com.google.common.truth.Truth.assertThat
import io.github.norbertque.pixeltone.playback.PlaybackState
import io.github.norbertque.pixeltone.playback.PlayerControls
import io.github.norbertque.pixeltone.playback.QueueEntry
import io.github.norbertque.pixeltone.ui.theme.PixeltoneTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = RobolectricDeviceQualifiers.Pixel7, sdk = [35])
class GesturesTest {
    @get:Rule
    val compose = createComposeRule()

    private val calls = mutableListOf<String>()

    private val controls = object : PlayerControls {
        override fun toggle() {
            calls += "toggle"
        }

        override fun next() {
            calls += "next"
        }

        override fun previous() {
            calls += "previous"
        }

        override fun seekTo(positionMs: Long) {}
        override fun jumpTo(index: Int) {}
        override fun move(from: Int, to: Int) {}
        override fun remove(index: Int) {}
        override fun cycleRepeat() {}
        override fun toggleShuffle() {}
    }

    // Paused: the seek bar's wave stands still, so the test's clock runs by itself.
    private val state = PlaybackState(
        queue = listOf(QueueEntry(1, "First", "Somebody", "An album", null), QueueEntry(2, "Second", "Somebody", "An album", null)),
        index = 0,
        durationMs = 200_000,
    )

    private fun nowPlaying(onBack: () -> Unit = {}) = compose.setContent {
        PixeltoneTheme { NowPlayingContent(state, 0, controls, onBack, lyrics = { Text("The lyrics") }) }
    }

    /** Sideways across the cover, which fills the top of the screen under the header. */
    private fun swipeOnCover(fromX: Float, toX: Float) = compose.onRoot().performTouchInput {
        swipe(Offset(width * fromX, height * 0.3f), Offset(width * toX, height * 0.3f), durationMillis = 200)
    }

    @Test
    fun aSwipeLeftOnTheCoverPlaysTheNextTrack() {
        nowPlaying()
        swipeOnCover(0.8f, 0.1f)
        compose.waitForIdle()
        assertThat(calls).containsExactly("next")
    }

    @Test
    fun aSwipeRightPlaysThePreviousOne() {
        nowPlaying()
        swipeOnCover(0.2f, 0.9f)
        compose.waitForIdle()
        assertThat(calls).containsExactly("previous")
    }

    @Test
    fun aShortSwipeSkipsNothing() {
        nowPlaying()
        swipeOnCover(0.5f, 0.4f)
        compose.waitForIdle()
        assertThat(calls).isEmpty()
    }

    @Test
    fun aSwipeDownCloses() {
        var closed = false
        nowPlaying(onBack = { closed = true })
        compose.onRoot().performTouchInput { swipeDown(startY = height * 0.3f, endY = height * 0.8f) }
        compose.waitForIdle()
        assertThat(closed).isTrue()
    }

    @Test
    fun aSwipeUpPullsUpTheLyrics() {
        nowPlaying()
        compose.onNodeWithText("The lyrics").assertDoesNotExist()
        compose.onRoot().performTouchInput { swipeUp(startY = height * 0.6f, endY = height * 0.2f) }
        compose.waitForIdle()
        compose.onNodeWithText("The lyrics").assertIsDisplayed()
    }

    @Test
    fun theQueueTabPullsUpTheQueue() {
        nowPlaying()
        compose.onNodeWithText("Queue").performClick()
        compose.waitForIdle()
        compose.onNodeWithText("2 tracks").assertIsDisplayed()
    }

    @Test
    fun thePanelComesUpWhereItWasLeft() {
        val picked = mutableListOf<NowPlayingPanel>()
        compose.setContent {
            PixeltoneTheme {
                NowPlayingContent(state, 0, controls, {}, lyrics = { Text("The lyrics") }, firstPanel = NowPlayingPanel.Queue, onPanel = { picked += it })
            }
        }
        compose.onRoot().performTouchInput { swipeUp(startY = height * 0.6f, endY = height * 0.2f) }
        compose.waitForIdle()
        compose.onNodeWithText("2 tracks").assertIsDisplayed()
        compose.onNodeWithText("Lyrics").performClick()
        compose.waitForIdle()
        compose.onNodeWithText("The lyrics").assertIsDisplayed()
        assertThat(picked.last()).isEqualTo(NowPlayingPanel.Lyrics)
    }

    @Test
    fun theMiniPlayerSkipsSidewaysAndOpensUpwards() {
        var opened = false
        compose.setContent {
            PixeltoneTheme {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    MiniPlayer(state.queue[0], isPlaying = false, onToggle = {}, onOpen = { opened = true }, onNext = controls::next, onPrevious = controls::previous)
                }
            }
        }
        compose.onRoot().performTouchInput { swipeLeft(startX = width * 0.9f, endX = width * 0.1f) }
        compose.waitForIdle()
        assertThat(calls).containsExactly("next")
        compose.onRoot().performTouchInput { swipeUp(startY = centerY, endY = centerY - 300f) }
        compose.waitForIdle()
        assertThat(opened).isTrue()
    }
}
