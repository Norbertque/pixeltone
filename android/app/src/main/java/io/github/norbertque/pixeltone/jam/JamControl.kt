package io.github.norbertque.pixeltone.jam

import android.content.Context
import android.provider.OpenableColumns
import androidx.core.content.edit
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.library.trackUri
import io.github.norbertque.pixeltone.remote.JamInvite
import io.github.norbertque.pixeltone.remote.JoinOutcome
import io.github.norbertque.pixeltone.remote.PairedPc
import io.github.norbertque.pixeltone.remote.PcCommand
import io.github.norbertque.pixeltone.remote.PcConnection
import io.github.norbertque.pixeltone.remote.RemoteControl
import io.github.norbertque.pixeltone.remote.UploadSource
import java.io.FileNotFoundException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** Where this phone stands in someone else's jam. */
sealed interface GuestStep {
    val invite: JamInvite

    /** A jam's QR code was opened: join with a name? [error] when the last try failed. */
    data class Invited(override val invite: JamInvite, val busy: Boolean = false, val error: String? = null) : GuestStep

    /** Removed earlier: the owner has to let this phone back in. */
    data class Waiting(override val invite: JamInvite) : GuestStep

    /** In the jam: [RemoteControl.guestConnection] follows it. */
    data class In(override val invite: JamInvite) : GuestStep

    /** Over for this phone: [message] says why. */
    data class Over(override val invite: JamInvite, val message: String) : GuestStep
}

/** Phone tracks on their way to a jam. */
data class Uploads(
    val total: Int = 0,
    val done: Int = 0,
    val current: String? = null,
    /** Of the current track. */
    val fraction: Float = 0f,
) {
    val active: Boolean get() = done < total
}

/**
 * Jams on this phone: joining someone's as a guest, and sending phone tracks to a jam, as a
 * guest or as its owner (the PC keeps copies until the jam ends; a hand-over isn't possible
 * then).
 */
class JamControl(private val context: Context, private val remote: RemoteControl) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val prefs = context.getSharedPreferences("jam", Context.MODE_PRIVATE)

    private val step = MutableStateFlow<GuestStep?>(null)
    val guest: StateFlow<GuestStep?> = step.asStateFlow()

    private val sending = MutableStateFlow(Uploads())
    val uploads: StateFlow<Uploads> = sending.asStateFlow()

    private val reports = MutableSharedFlow<String>(extraBufferCapacity = 8, onBufferOverflow = BufferOverflow.DROP_OLDEST)

    /** Uploads that failed, in words for the user. */
    val errors: SharedFlow<String> = reports.asSharedFlow()

    private val queue = Channel<Pending>(Channel.UNLIMITED)
    private var waiting: Job? = null

    /** The name guests go by, remembered from the last jam. */
    var name: String
        get() = prefs.getString(NAME, "").orEmpty()
        private set(value) = prefs.edit { putString(NAME, value) }

    init {
        scope.launch {
            remote.guestConnection.jamClosed.collect { reason ->
                val invite = step.value?.invite ?: return@collect
                step.value = GuestStep.Over(invite, closedMessage(reason, invite.pcName))
            }
        }
        scope.launch {
            for (pending in queue) send(pending)
        }
    }

    /** A jam's link was opened. Already in that jam: nothing changes. */
    fun open(invite: JamInvite) {
        val now = step.value
        if (now is GuestStep.In && now.invite == invite) return
        if (now is GuestStep.In) leave()
        waiting?.cancel()
        step.value = GuestStep.Invited(invite)
    }

    fun join(name: String) {
        val invited = step.value as? GuestStep.Invited ?: return
        this.name = name.trim()
        step.value = invited.copy(busy = true, error = null)
        scope.launch { follow(invited.invite, remote.guestConnection.join(invited.invite, name.trim())) }
    }

    private suspend fun follow(invite: JamInvite, outcome: JoinOutcome) {
        when (outcome) {
            is JoinOutcome.In -> {
                remote.guestConnection.begin(outcome.pc)
                step.value = GuestStep.In(invite)
            }
            is JoinOutcome.Waiting -> {
                step.value = GuestStep.Waiting(invite)
                waiting = scope.launch {
                    var answer: JoinOutcome = outcome
                    while (answer is JoinOutcome.Waiting) {
                        delay(POLL_MS)
                        answer = remote.guestConnection.rejoinStatus(invite, outcome.requestId)
                    }
                    follow(invite, answer)
                }
            }
            is JoinOutcome.Refused -> step.update { now ->
                when (now) {
                    // The owner said no, or the request ran out: shown on its own.
                    is GuestStep.Waiting -> GuestStep.Over(invite, outcome.message)
                    else -> GuestStep.Invited(invite, error = outcome.message)
                }
            }
        }
    }

    /** Out of the jam; the tracks this phone added stay in it. */
    fun leave() {
        waiting?.cancel()
        remote.guestConnection.send(PcCommand.simple("jamLeave"))
        remote.guestConnection.end()
        step.value = null
    }

    /** Closes whatever the guest screen shows (an invite not taken, a jam that's over). */
    fun dismiss() {
        if (step.value is GuestStep.In) return leave()
        waiting?.cancel()
        step.value = null
    }

    /** Sends phone tracks to the jam on [via]'s PC, one after another. */
    fun send(tracks: List<Track>, via: PcConnection) {
        val pc = via.session.value?.pc ?: return
        if (tracks.isEmpty()) return
        sending.update { if (it.active) it.copy(total = it.total + tracks.size) else Uploads(total = tracks.size) }
        tracks.forEach { queue.trySend(Pending(it, via, pc)) }
    }

    private suspend fun send(pending: Pending) {
        val track = pending.track
        sending.update { it.copy(current = track.title, fraction = 0f) }
        try {
            val source = uploadSource(track)
            pending.via.upload(pending.pc, source) { sent ->
                sending.update { it.copy(fraction = (sent.toFloat() / source.size.coerceAtLeast(1)).coerceIn(0f, 1f)) }
            }
        } catch (e: Exception) {
            reports.tryEmit("“${track.title}” wasn't sent: ${e.message ?: e.javaClass.simpleName}")
        } finally {
            sending.update { it.copy(done = it.done + 1, current = null, fraction = 0f) }
        }
    }

    private fun uploadSource(track: Track): UploadSource {
        val uri = trackUri(track.id)
        val resolver = context.contentResolver
        var name = "${track.title}.bin"
        var size = -1L
        resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)?.use { c ->
            if (c.moveToFirst()) {
                c.getString(0)?.let { name = it }
                if (!c.isNull(1)) size = c.getLong(1)
            }
        }
        if (size < 0) {
            size = resolver.openAssetFileDescriptor(uri, "r")?.use { it.length.takeIf { l -> l >= 0 } ?: it.parcelFileDescriptor.statSize }
                ?: throw FileNotFoundException(track.title)
        }
        return UploadSource(
            name = name,
            size = size,
            mimeType = resolver.getType(uri) ?: "application/octet-stream",
            title = track.title,
            artist = track.artist,
            album = track.album,
            durationMs = track.durationMs.takeIf { it > 0 },
            open = { resolver.openInputStream(uri) ?: throw FileNotFoundException(track.title) },
        )
    }

    private class Pending(val track: Track, val via: PcConnection, val pc: PairedPc)

    private companion object {
        const val NAME = "name"
        const val POLL_MS = 3_000L

        fun closedMessage(reason: String, pc: String): String = when (reason) {
            "ended" -> "The owner ended the jam on $pc."
            "removed" -> "The owner removed you from the jam."
            else -> "You're no longer in the jam on $pc."
        }
    }
}
