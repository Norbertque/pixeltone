package io.github.norbertque.pixeltone.ui

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.update.UpdateInfo
import io.github.norbertque.pixeltone.update.UpdateState

/** Offers a newer build and shows its download; nothing while there's none. */
@Composable
fun UpdateDialog(state: UpdateState, currentVersion: String, onUpdate: (UpdateInfo) -> Unit, onDismiss: () -> Unit) {
    when (state) {
        UpdateState.Idle, UpdateState.Installing -> Unit
        is UpdateState.Available -> AlertDialog(
            onDismissRequest = onDismiss,
            title = { Text(stringResource(R.string.update_title)) },
            text = { Text(stringResource(R.string.update_text, state.info.versionName, currentVersion)) },
            confirmButton = { TextButton(onClick = { onUpdate(state.info) }) { Text(stringResource(R.string.update_now)) } },
            dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.update_later)) } },
        )
        is UpdateState.Downloading -> AlertDialog(
            onDismissRequest = {},
            title = { Text(stringResource(R.string.update_downloading, state.info.versionName)) },
            text = { LinearProgressIndicator(progress = { state.progress }, modifier = Modifier.fillMaxWidth()) },
            confirmButton = {},
        )
        is UpdateState.Failed -> AlertDialog(
            onDismissRequest = onDismiss,
            title = { Text(stringResource(R.string.update_failed)) },
            text = { Text(state.message) },
            confirmButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.ok)) } },
        )
    }
}
