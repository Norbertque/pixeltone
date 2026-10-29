package io.github.norbertque.pixeltone.ui.player

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.BedtimeOff
import androidx.compose.material.icons.filled.Timer
import androidx.compose.material.icons.filled.TrackChanges
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.playback.Sleep
import kotlinx.coroutines.delay

private val MINUTES = listOf(15, 30, 45, 60)

/** When to stop: in so many minutes, at the end of the track, or not at all. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SleepSheet(sleep: Sleep, onMinutes: (Int) -> Unit, onEndOfTrack: () -> Unit, onCancel: () -> Unit, onDismiss: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.navigationBarsPadding()) {
            Text(
                stringResource(R.string.sleep_timer),
                Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                style = MaterialTheme.typography.titleMedium,
            )
            fun pick(action: () -> Unit) {
                action()
                onDismiss()
            }
            MINUTES.forEach { minutes ->
                Option(Icons.Default.Timer, pluralStringResource(R.plurals.sleep_minutes, minutes, minutes)) { pick { onMinutes(minutes) } }
            }
            Option(Icons.Default.TrackChanges, stringResource(R.string.sleep_end_of_track)) { pick(onEndOfTrack) }
            if (sleep != Sleep.Off) Option(Icons.Default.BedtimeOff, stringResource(R.string.sleep_off)) { pick(onCancel) }
        }
    }
}

@Composable
private fun Option(icon: ImageVector, label: String, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(label) },
        leadingContent = { Icon(icon, null) },
        modifier = Modifier.clickable(onClick = onClick),
    )
}

/** "Stops in 23:41", "Stops after this track"; null when the timer is off. */
@Composable
fun sleepText(sleep: Sleep, now: () -> Long = System::currentTimeMillis): String? {
    var current by remember { mutableLongStateOf(now()) }
    LaunchedEffect(sleep) {
        while (sleep is Sleep.At) {
            current = now()
            delay(1_000)
        }
    }
    return when (sleep) {
        Sleep.Off -> null
        Sleep.EndOfTrack -> stringResource(R.string.sleep_after_track)
        is Sleep.At -> {
            val left = ((sleep.endsAt - current).coerceAtLeast(0) + 999) / 1000
            stringResource(R.string.sleep_in, "%d:%02d".format(left / 60, left % 60))
        }
    }
}

