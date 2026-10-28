package io.github.norbertque.pixeltone.remote

import java.io.File
import java.util.UUID
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.Serializable

/** A PC this phone paired with. `token` is the phone's key to it (keep it secret). */
@Serializable
data class PairedPc(
    val id: String,
    val name: String,
    val host: String,
    val port: Int,
    val fingerprint: String,
    val token: String,
)

@Serializable
private data class PairedPcsFile(val deviceId: String, val pcs: List<PairedPc> = emptyList())

/**
 * Paired PCs and this phone's id towards them, in a file of the app's no-backup folder:
 * private to the app, and never copied into a cloud backup with the tokens in it.
 */
class PairedPcs(private val file: File) {
    // A new phone id is written with the first paired PC, the first time anyone needs it.
    private var data: PairedPcsFile =
        runCatching { RemoteJson.decodeFromString(PairedPcsFile.serializer(), file.readText()) }.getOrNull()
            ?: PairedPcsFile(deviceId = UUID.randomUUID().toString())
    private val current = MutableStateFlow(data.pcs)

    val pcs: StateFlow<List<PairedPc>> = current.asStateFlow()

    /** This phone's id towards PCs, made once. */
    val deviceId: String get() = data.deviceId

    @Synchronized
    fun find(id: String): PairedPc? = data.pcs.firstOrNull { it.id == id }

    fun save(pc: PairedPc) = update { list -> list.filterNot { it.id == pc.id } + pc }

    /** The PC turned up at another address (DHCP, another port). */
    fun moved(id: String, host: String, port: Int) = update { list ->
        list.map { if (it.id == id && (it.host != host || it.port != port)) it.copy(host = host, port = port) else it }
    }

    fun forget(id: String) = update { list -> list.filterNot { it.id == id } }

    @Synchronized
    private fun update(change: (List<PairedPc>) -> List<PairedPc>) {
        val updated = change(data.pcs)
        if (updated == data.pcs) return
        data = data.copy(pcs = updated)
        write()
        current.value = updated
    }

    private fun write() {
        file.parentFile?.mkdirs()
        val partial = File(file.parentFile, file.name + ".part")
        partial.writeText(RemoteJson.encodeToString(PairedPcsFile.serializer(), data))
        partial.renameTo(file)
    }
}
