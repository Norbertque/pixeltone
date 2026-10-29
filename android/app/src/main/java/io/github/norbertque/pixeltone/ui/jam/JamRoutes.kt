package io.github.norbertque.pixeltone.ui.jam

import android.content.Intent
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.jam.GuestStep
import io.github.norbertque.pixeltone.library.AUDIO_PERMISSION
import io.github.norbertque.pixeltone.library.mayReadAudio
import io.github.norbertque.pixeltone.remote.JamGuest
import io.github.norbertque.pixeltone.remote.PcCommand
import io.github.norbertque.pixeltone.remote.PcConnection
import io.github.norbertque.pixeltone.remote.PcCover
import io.github.norbertque.pixeltone.remote.PcLibraryAlbum
import io.github.norbertque.pixeltone.remote.PcLibraryAlbumDetail
import io.github.norbertque.pixeltone.remote.PcLibraryResults
import io.github.norbertque.pixeltone.remote.PcQueueRow
import io.github.norbertque.pixeltone.remote.RemoteSession
import kotlinx.coroutines.flow.merge

/** Someone else's jam on this phone: joining it, waiting to be let back in, in it, or over. */
@Composable
fun GuestJamScreen(app: PixeltoneApp) {
    val control = app.jam
    val step by control.guest.collectAsStateWithLifecycle()
    when (val s = step) {
        null -> Unit
        is GuestStep.Invited -> {
            BackHandler(onBack = control::dismiss)
            JoinContent(s.invite.pcName, control.name, s.busy, s.error, control::join, control::dismiss)
        }
        is GuestStep.Waiting -> {
            BackHandler(onBack = control::dismiss)
            WaitingContent(onCancel = control::dismiss)
        }
        is GuestStep.Over -> {
            BackHandler(onBack = control::dismiss)
            OverContent(s.message, control::dismiss)
        }
        is GuestStep.In -> InJamAsGuest(app, s.invite.pcName)
    }
}

@Composable
private fun InJamAsGuest(app: PixeltoneApp, pcName: String) {
    val connection = app.remote.guestConnection
    val session by connection.session.collectAsStateWithLifecycle()
    val uploads by app.jam.uploads.collectAsStateWithLifecycle()
    var leaving by rememberSaveable { mutableStateOf(false) }
    var adding by rememberSaveable { mutableStateOf(false) }
    val messages = remember { SnackbarHostState() }
    LaunchedEffect(Unit) { merge(connection.errors, app.jam.errors).collect { messages.showSnackbar(it) } }
    BackHandler { leaving = true }
    val (playing, upNext) = rememberQueue(connection, session)
    JamContent(
        title = stringResource(R.string.jam_of, session?.pc?.name ?: pcName),
        connected = session?.connected == true,
        state = session?.state,
        playing = playing,
        upNext = upNext,
        me = session?.guest,
        owner = null,
        uploads = uploads,
        coverOf = { PcCover(it, guest = true) },
        onBack = { leaving = true },
        onFinish = { leaving = true },
        onCommand = { connection.send(it) },
        onAddTracks = { adding = true },
        messages = messages,
    )
    if (adding) AddTracks(app, connection, session) { adding = false }
    if (leaving) {
        ConfirmDialog(
            title = stringResource(R.string.jam_leave_title),
            text = stringResource(R.string.jam_leave_text),
            confirm = stringResource(R.string.jam_leave),
            onConfirm = {
                leaving = false
                app.jam.leave()
            },
            onDismiss = { leaving = false },
            dismiss = stringResource(R.string.jam_stay),
        )
    }
}

/** The jam on the PC that playback is on, from the owner's phone: start it, or run it. */
@Composable
fun OwnerJamScreen(app: PixeltoneApp, onBack: () -> Unit) {
    val context = LocalContext.current
    val connection = app.remote.connection
    val session by connection.session.collectAsStateWithLifecycle()
    val uploads by app.jam.uploads.collectAsStateWithLifecycle()
    var adding by rememberSaveable { mutableStateOf(false) }
    var ending by rememberSaveable { mutableStateOf(false) }
    var removing by remember { mutableStateOf<JamGuest?>(null) }
    val messages = remember { SnackbarHostState() }
    LaunchedEffect(Unit) { app.jam.errors.collect { messages.showSnackbar(it) } }
    val current = session
    if (current == null) {
        // No longer on a PC.
        LaunchedEffect(Unit) { onBack() }
        return
    }
    val jam = current.jam
    if (jam == null) {
        StartJamContent(current.pc.name, onStart = { connection.send(PcCommand.simple("jamStart")) }, onBack = onBack)
        return
    }
    val (playing, upNext) = rememberQueue(connection, current)
    val shareTitle = stringResource(R.string.jam_share_title)
    JamContent(
        title = stringResource(R.string.jam),
        connected = current.connected,
        state = current.state,
        playing = playing,
        upNext = upNext,
        me = null,
        owner = OwnerPart(
            jam = jam,
            onShare = {
                val send = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, jam.link)
                context.startActivity(Intent.createChooser(send, shareTitle))
            },
            onRemoveGuest = { removing = it },
            onPermissions = { connection.send(PcCommand.jamPermissions(it)) },
            onAnswer = { id, allow -> connection.send(PcCommand.jamAnswer(id, allow)) },
        ),
        uploads = uploads,
        coverOf = { PcCover(it) },
        onBack = onBack,
        onFinish = { ending = true },
        onCommand = { connection.send(it) },
        onAddTracks = { adding = true },
        messages = messages,
    )
    if (adding) AddTracks(app, connection, current) { adding = false }
    removing?.let { guest ->
        RemoveGuestDialog(
            guest,
            upcoming = upNext.count { it.guestId == guest.id },
            onConfirm = {
                removing = null
                connection.send(PcCommand.jamRemoveGuest(guest.id))
            },
            onDismiss = { removing = null },
        )
    }
    if (ending) {
        ConfirmDialog(
            title = stringResource(R.string.jam_end_title),
            text = stringResource(R.string.jam_end_text),
            confirm = stringResource(R.string.jam_end),
            onConfirm = {
                ending = false
                connection.send(PcCommand.simple("jamEnd"))
            },
            onDismiss = { ending = false },
        )
    }
}

@Composable
private fun AddTracks(app: PixeltoneApp, connection: PcConnection, session: RemoteSession?, onDismiss: () -> Unit) {
    val pc = session?.pc ?: return onDismiss()
    val context = LocalContext.current
    var allowed by remember { mutableStateOf(context.mayReadAudio()) }
    val ask = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        allowed = granted
        if (granted) app.library.start()
    }
    LaunchedEffect(allowed) { if (allowed) app.library.start() }
    val index by app.library.index.collectAsStateWithLifecycle()
    val library = remember(pc) {
        object : PcLibrary {
            override suspend fun search(query: String): PcLibraryResults = connection.librarySearch(pc, query)

            override suspend fun albums(): List<PcLibraryAlbum> = connection.libraryAlbums(pc, 0, 1000)

            override suspend fun album(id: Long): PcLibraryAlbumDetail = connection.libraryAlbum(pc, id)
        }
    }
    val guest = connection === app.remote.guestConnection
    AddTracksSheet(
        pcName = pc.name,
        pc = library,
        coverOf = { PcCover(it, guest) },
        phone = index,
        phoneAllowed = allowed,
        onAllowPhone = { ask.launch(AUDIO_PERMISSION) },
        onAddFromPc = { connection.send(PcCommand.queueAdd(it)) },
        onSendFromPhone = { app.jam.send(it, connection) },
        onDismiss = onDismiss,
    )
}

/** The queue row that plays and those after it, fetched again whenever the queue changes. */
@Composable
private fun rememberQueue(connection: PcConnection, session: RemoteSession?): Pair<PcQueueRow?, List<PcQueueRow>> {
    var rows by remember { mutableStateOf<Pair<PcQueueRow?, List<PcQueueRow>>>(null to emptyList()) }
    val pc = session?.pc
    val state = session?.state
    LaunchedEffect(pc?.id, state?.queueVersion, state?.queueIndex, session?.connected) {
        if (pc == null || session.connected != true || state == null) return@LaunchedEffect
        val from = state.queueIndex ?: 0
        runCatching { connection.queue(pc, from, UP_NEXT) }.onSuccess { page ->
            rows = if (state.queueIndex != null) page.firstOrNull() to page.drop(1) else null to page
        }
    }
    return rows
}

private const val UP_NEXT = 300
