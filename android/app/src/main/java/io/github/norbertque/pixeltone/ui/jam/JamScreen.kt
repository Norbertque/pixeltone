package io.github.norbertque.pixeltone.ui.jam

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.PlaylistAdd
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Groups
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.PersonAdd
import androidx.compose.material.icons.filled.PersonRemove
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.SkipNext
import androidx.compose.material.icons.filled.SkipPrevious
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.jam.Uploads
import io.github.norbertque.pixeltone.remote.GuestPermissions
import io.github.norbertque.pixeltone.remote.GuestView
import io.github.norbertque.pixeltone.remote.JamGuest
import io.github.norbertque.pixeltone.remote.PcCommand
import io.github.norbertque.pixeltone.remote.PcJam
import io.github.norbertque.pixeltone.remote.PcQueueRow
import io.github.norbertque.pixeltone.remote.PcState
import io.github.norbertque.pixeltone.ui.BackButton
import io.github.norbertque.pixeltone.ui.PixeltoneGlyph
import io.github.norbertque.pixeltone.ui.formatDuration
import io.github.norbertque.pixeltone.ui.library.Cover

/* ------------------------------ Before joining ------------------------------ */

@Composable
fun JoinContent(
    pcName: String,
    name: String,
    busy: Boolean,
    error: String?,
    onJoin: (String) -> Unit,
    onCancel: () -> Unit,
) {
    var typed by rememberSaveable { mutableStateOf(name) }
    val join = { if (typed.isNotBlank() && !busy) onJoin(typed) }
    Centered {
        Icon(Icons.Default.Groups, null, Modifier.size(56.dp), tint = MaterialTheme.colorScheme.primary)
        Spacer(Modifier.height(24.dp))
        Text(stringResource(R.string.jam_join_title, pcName), style = MaterialTheme.typography.headlineSmall, textAlign = TextAlign.Center)
        Spacer(Modifier.height(12.dp))
        Text(
            stringResource(R.string.jam_join_text, pcName),
            textAlign = TextAlign.Center,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(24.dp))
        OutlinedTextField(
            value = typed,
            onValueChange = { typed = it.take(40) },
            label = { Text(stringResource(R.string.jam_your_name)) },
            singleLine = true,
            enabled = !busy,
            isError = error != null,
            supportingText = error?.let { { Text(it) } },
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Words, imeAction = ImeAction.Go),
            keyboardActions = KeyboardActions(onGo = { join() }),
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(Modifier.height(16.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            TextButton(onClick = onCancel) { Text(stringResource(R.string.cancel)) }
            Button(onClick = join, enabled = typed.isNotBlank() && !busy) {
                if (busy) CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp) else Text(stringResource(R.string.jam_join))
            }
        }
    }
}

@Composable
fun WaitingContent(onCancel: () -> Unit) {
    Centered {
        CircularProgressIndicator()
        Spacer(Modifier.height(24.dp))
        Text(stringResource(R.string.jam_waiting_title), style = MaterialTheme.typography.headlineSmall)
        Spacer(Modifier.height(12.dp))
        Text(
            stringResource(R.string.jam_waiting_text),
            textAlign = TextAlign.Center,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(24.dp))
        TextButton(onClick = onCancel) { Text(stringResource(R.string.cancel)) }
    }
}

@Composable
fun OverContent(message: String, onClose: () -> Unit) {
    Centered {
        PixeltoneGlyph(Modifier.size(56.dp))
        Spacer(Modifier.height(24.dp))
        Text(stringResource(R.string.jam_over_title), style = MaterialTheme.typography.headlineSmall)
        Spacer(Modifier.height(12.dp))
        Text(message, textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Spacer(Modifier.height(24.dp))
        Button(onClick = onClose) { Text(stringResource(R.string.jam_close)) }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StartJamContent(pcName: String, onStart: () -> Unit, onBack: () -> Unit) {
    Scaffold(topBar = { TopAppBar(title = { Text(stringResource(R.string.jam)) }, navigationIcon = { BackButton(onBack) }) }) { padding ->
        Box(Modifier.padding(padding)) {
            Centered(safe = false) {
                Icon(Icons.Default.Groups, null, Modifier.size(56.dp), tint = MaterialTheme.colorScheme.primary)
                Spacer(Modifier.height(24.dp))
                Text(stringResource(R.string.jam_start_title), style = MaterialTheme.typography.headlineSmall)
                Spacer(Modifier.height(12.dp))
                Text(
                    stringResource(R.string.jam_start_text, pcName),
                    textAlign = TextAlign.Center,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.height(24.dp))
                Button(onClick = onStart) {
                    Icon(Icons.Default.Groups, null, Modifier.size(18.dp))
                    Spacer(Modifier.size(8.dp))
                    Text(stringResource(R.string.jam_start))
                }
            }
        }
    }
}

@Composable
private fun Centered(safe: Boolean = true, content: @Composable () -> Unit) {
    Surface(Modifier.fillMaxSize()) {
        Box(
            (if (safe) Modifier.safeDrawingPadding() else Modifier).fillMaxSize().padding(32.dp),
            contentAlignment = Alignment.Center,
        ) {
            Column(Modifier.widthIn(max = 420.dp), horizontalAlignment = Alignment.CenterHorizontally) { content() }
        }
    }
}

/* -------------------------------- In the jam -------------------------------- */

/** What the owner sees of the jam besides the queue; null on a guest's phone. */
class OwnerPart(
    val jam: PcJam,
    val onShare: () -> Unit,
    val onRemoveGuest: (JamGuest) -> Unit,
    val onPermissions: (GuestPermissions) -> Unit,
    val onAnswer: (requestId: String, allow: Boolean) -> Unit,
)

/**
 * A jam: the owner's QR code, guests and guest permissions ([owner]), then what plays and the
 * shared queue with who added what. [me] is this phone's place in it as a guest.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun JamContent(
    title: String,
    connected: Boolean,
    state: PcState?,
    /** The queue row that plays, and those after it. */
    playing: PcQueueRow?,
    upNext: List<PcQueueRow>,
    me: GuestView?,
    owner: OwnerPart?,
    uploads: Uploads,
    coverOf: (String) -> Any,
    onBack: () -> Unit,
    onFinish: () -> Unit,
    onCommand: (String) -> Unit,
    onAddTracks: () -> Unit,
    messages: SnackbarHostState? = null,
) {
    val permissions = me?.permissions
    val canAdd = permissions?.addTracks ?: true
    val canPlay = permissions?.controlPlayback ?: true
    Scaffold(
        snackbarHost = { messages?.let { SnackbarHost(it) } },
        topBar = {
            TopAppBar(
                title = { Text(title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = { BackButton(onBack) },
                actions = {
                    TextButton(onClick = onFinish) {
                        Text(stringResource(if (owner != null) R.string.jam_end else R.string.jam_leave))
                    }
                },
            )
        },
        floatingActionButton = {
            if (canAdd) {
                ExtendedFloatingActionButton(
                    onClick = onAddTracks,
                    icon = { Icon(Icons.AutoMirrored.Filled.PlaylistAdd, null) },
                    text = { Text(stringResource(R.string.jam_add_tracks)) },
                )
            }
        },
    ) { padding ->
        LazyColumn(
            Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(bottom = 96.dp),
        ) {
            if (!connected) {
                item { Text(stringResource(R.string.jam_reconnecting), Modifier.padding(16.dp, 4.dp), color = MaterialTheme.colorScheme.error) }
            }
            if (owner != null) ownerSection(owner)
            item { NowPlayingCard(state, playing, coverOf, canPlay, onCommand) }
            if (uploads.active) item { UploadCard(uploads) }
            item { SectionTitle(stringResource(R.string.jam_up_next)) }
            items(upNext, key = { it.id }) { row ->
                val mayChange = owner != null ||
                    (me != null && row.guestId == me.guestId && me.permissions.changeOwnTracks)
                QueueRow(row, coverOf, onRemove = if (mayChange) ({ onCommand(PcCommand.queueRemove(row.id)) }) else null)
            }
        }
    }
}

private fun androidx.compose.foundation.lazy.LazyListScope.ownerSection(owner: OwnerPart) {
    val jam = owner.jam
    item {
        Column(Modifier.fillMaxWidth().padding(16.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            QrCode(jam.link, stringResource(R.string.jam_qr), Modifier.widthIn(max = 260.dp).fillMaxWidth())
            Spacer(Modifier.height(12.dp))
            Text(
                stringResource(R.string.jam_qr_hint),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(8.dp))
            OutlinedButton(onClick = owner.onShare) {
                Icon(Icons.Default.Share, null, Modifier.size(18.dp))
                Spacer(Modifier.size(8.dp))
                Text(stringResource(R.string.jam_share))
            }
        }
    }
    if (jam.requests.isNotEmpty()) {
        item { SectionTitle(stringResource(R.string.jam_requests)) }
        items(jam.requests, key = { "request-${it.id}" }) { request ->
            Card(
                Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.tertiaryContainer),
            ) {
                Row(Modifier.padding(start = 16.dp, end = 8.dp, top = 8.dp, bottom = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.PersonAdd, null)
                    Spacer(Modifier.size(12.dp))
                    Text(stringResource(R.string.jam_request, request.name), Modifier.weight(1f))
                    TextButton(
                        onClick = { owner.onAnswer(request.id, false) },
                        colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.onTertiaryContainer),
                    ) { Text(stringResource(R.string.jam_decline)) }
                    FilledTonalButton(onClick = { owner.onAnswer(request.id, true) }) { Text(stringResource(R.string.jam_allow)) }
                }
            }
        }
    }
    item {
        Row(Modifier.padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 4.dp), verticalAlignment = Alignment.Bottom) {
            Text(stringResource(R.string.jam_guests), style = MaterialTheme.typography.titleMedium)
            Spacer(Modifier.size(8.dp))
            Text(
                stringResource(R.string.jam_guests_count, jam.guests.size),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
    if (jam.guests.isEmpty()) {
        item {
            Text(
                stringResource(R.string.jam_no_guests),
                Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
    items(jam.guests, key = { "guest-${it.id}" }) { guest ->
        ListItem(
            headlineContent = { Text(guest.name) },
            leadingContent = { Icon(Icons.Default.Person, null) },
            trailingContent = {
                IconButton(onClick = { owner.onRemoveGuest(guest) }) {
                    Icon(Icons.Default.PersonRemove, stringResource(R.string.jam_remove_guest, guest.name))
                }
            },
        )
    }
    item { SectionTitle(stringResource(R.string.jam_permissions)) }
    item {
        val p = jam.permissions
        Column {
            PermissionRow(stringResource(R.string.jam_perm_add), stringResource(R.string.jam_perm_add_text), p.addTracks) {
                owner.onPermissions(p.copy(addTracks = it))
            }
            PermissionRow(stringResource(R.string.jam_perm_change), stringResource(R.string.jam_perm_change_text), p.changeOwnTracks) {
                owner.onPermissions(p.copy(changeOwnTracks = it))
            }
            PermissionRow(stringResource(R.string.jam_perm_playback), stringResource(R.string.jam_perm_playback_text), p.controlPlayback) {
                owner.onPermissions(p.copy(controlPlayback = it))
            }
        }
    }
}

@Composable
private fun PermissionRow(title: String, text: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    ListItem(
        headlineContent = { Text(title) },
        supportingContent = { Text(text) },
        trailingContent = { Switch(checked = checked, onCheckedChange = onChange) },
    )
}

@Composable
private fun SectionTitle(text: String) {
    Text(
        text,
        Modifier.padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 4.dp),
        style = MaterialTheme.typography.titleMedium,
    )
}

@Composable
private fun NowPlayingCard(
    state: PcState?,
    row: PcQueueRow?,
    coverOf: (String) -> Any,
    canPlay: Boolean,
    onCommand: (String) -> Unit,
) {
    val track = state?.track
    Card(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.secondaryContainer),
    ) {
        Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Cover(track?.coverKey?.let(coverOf), Modifier.size(64.dp))
            Spacer(Modifier.size(12.dp))
            Column(Modifier.weight(1f)) {
                if (track == null) {
                    Text(stringResource(R.string.jam_nothing_playing))
                } else {
                    Text(track.title, style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Text(
                        listOfNotNull(row?.addedBy, track.artist).joinToString(" · "),
                        style = MaterialTheme.typography.bodyMedium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
        if (canPlay && track != null) {
            Row(
                Modifier.fillMaxWidth().padding(bottom = 12.dp),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                IconButton(onClick = { onCommand(PcCommand.simple("previous")) }) {
                    Icon(Icons.Default.SkipPrevious, stringResource(R.string.previous))
                }
                FilledIconButton(onClick = { onCommand(PcCommand.simple("toggle")) }, Modifier.size(56.dp)) {
                    if (state.isPlaying) {
                        Icon(Icons.Default.Pause, stringResource(R.string.pause))
                    } else {
                        Icon(Icons.Default.PlayArrow, stringResource(R.string.play))
                    }
                }
                IconButton(onClick = { onCommand(PcCommand.simple("next")) }) {
                    Icon(Icons.Default.SkipNext, stringResource(R.string.next))
                }
            }
        }
    }
}

@Composable
private fun UploadCard(uploads: Uploads) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
        Text(
            stringResource(R.string.jam_sending, (uploads.done + 1).coerceAtMost(uploads.total), uploads.total, uploads.current.orEmpty()),
            style = MaterialTheme.typography.bodyMedium,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
        Spacer(Modifier.height(6.dp))
        LinearProgressIndicator(progress = { uploads.fraction }, Modifier.fillMaxWidth())
    }
}

@Composable
private fun QueueRow(row: PcQueueRow, coverOf: (String) -> Any, onRemove: (() -> Unit)?) {
    ListItem(
        headlineContent = { Text(row.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = {
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (row.addedBy != null) {
                    Icon(Icons.Default.Person, null, Modifier.size(14.dp), tint = MaterialTheme.colorScheme.tertiary)
                    Text(
                        row.addedBy,
                        color = MaterialTheme.colorScheme.tertiary,
                        fontWeight = FontWeight.Medium,
                        maxLines = 1,
                    )
                    Spacer(Modifier.size(6.dp))
                }
                Text(row.artist.orEmpty(), maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        },
        leadingContent = { Cover(row.coverKey?.let(coverOf), Modifier.size(48.dp)) },
        trailingContent = {
            when {
                onRemove != null -> IconButton(onClick = onRemove) {
                    Icon(Icons.Default.Close, stringResource(R.string.remove_from_queue))
                }
                row.durationMs != null -> Text(formatDuration(row.durationMs.toLong()))
            }
        },
        colors = ListItemDefaults.colors(),
    )
}

/* -------------------------------- Questions -------------------------------- */

@Composable
fun ConfirmDialog(
    title: String,
    text: String,
    confirm: String,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
    dismiss: String = stringResource(R.string.cancel),
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = { Text(text) },
        confirmButton = { TextButton(onClick = onConfirm) { Text(confirm) } },
        dismissButton = { TextButton(onClick = onDismiss) { Text(dismiss) } },
    )
}

/** "Remove Petr from the jam? Their 3 upcoming tracks will be removed too." */
@Composable
fun RemoveGuestDialog(guest: JamGuest, upcoming: Int, onConfirm: () -> Unit, onDismiss: () -> Unit) {
    ConfirmDialog(
        title = stringResource(R.string.jam_remove_title, guest.name),
        text = if (upcoming > 0) pluralStringResource(R.plurals.jam_remove_tracks, upcoming, upcoming) else "",
        confirm = stringResource(R.string.jam_remove),
        onConfirm = onConfirm,
        onDismiss = onDismiss,
    )
}
