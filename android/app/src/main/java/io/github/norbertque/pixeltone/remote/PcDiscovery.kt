package io.github.norbertque.pixeltone.remote

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Build
import java.net.Inet4Address
import java.net.InetAddress
import kotlin.coroutines.resume
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull

/** A Pixeltone PC announcing itself on the network. */
data class FoundPc(
    val serviceName: String,
    /** The PC's installation id (TXT `id`), the same as in its `/v1/info`. */
    val id: String?,
    val name: String,
    val host: String,
    val port: Int,
)

/** Finds Pixeltone PCs over mDNS / DNS-SD while something asks for it. */
class PcDiscovery(context: Context) {
    private val nsd: NsdManager? = context.getSystemService(NsdManager::class.java)
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val toResolve = Channel<NsdServiceInfo>(Channel.UNLIMITED)
    private val current = MutableStateFlow<List<FoundPc>>(emptyList())

    val found: StateFlow<List<FoundPc>> = current.asStateFlow()

    private var listener: NsdManager.DiscoveryListener? = null
    private var users = 0

    init {
        // One resolve at a time: older Android versions refuse a second one meanwhile.
        scope.launch {
            for (service in toResolve) {
                val resolved = resolve(service) ?: continue
                val pc = describe(resolved) ?: continue
                synchronized(this@PcDiscovery) {
                    if (listener == null) return@synchronized
                    current.value = (current.value.filterNot { it.serviceName == pc.serviceName } + pc)
                        .sortedBy { it.name.lowercase() }
                }
            }
        }
    }

    @Synchronized
    fun start() {
        users++
        if (listener != null) return
        val manager = nsd ?: return
        val discovery = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(serviceType: String) = Unit

            override fun onServiceFound(service: NsdServiceInfo) {
                toResolve.trySend(service)
            }

            override fun onServiceLost(service: NsdServiceInfo) {
                synchronized(this@PcDiscovery) {
                    current.value = current.value.filterNot { it.serviceName == service.serviceName }
                }
            }

            override fun onDiscoveryStopped(serviceType: String) = Unit

            override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                synchronized(this@PcDiscovery) {
                    if (listener === this) listener = null
                }
            }

            override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) = Unit
        }
        listener = discovery
        runCatching {
            manager.discoverServices(RemoteProtocol.SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, discovery)
        }.onFailure { listener = null }
    }

    @Synchronized
    fun stop() {
        users = (users - 1).coerceAtLeast(0)
        if (users > 0) return
        val discovery = listener ?: return
        listener = null
        runCatching { nsd?.stopServiceDiscovery(discovery) }
        current.value = emptyList()
    }

    @Suppress("DEPRECATION") // resolveService: still the way on every version this app runs on.
    private suspend fun resolve(service: NsdServiceInfo): NsdServiceInfo? {
        val manager = nsd ?: return null
        return withTimeoutOrNull(RESOLVE_TIMEOUT_MS) {
            suspendCancellableCoroutine { cont ->
                runCatching {
                    manager.resolveService(
                        service,
                        object : NsdManager.ResolveListener {
                            override fun onResolveFailed(info: NsdServiceInfo, errorCode: Int) {
                                if (cont.isActive) cont.resume(null)
                            }

                            override fun onServiceResolved(info: NsdServiceInfo) {
                                if (cont.isActive) cont.resume(info)
                            }
                        },
                    )
                }.onFailure { if (cont.isActive) cont.resume(null) }
            }
        }
    }

    private fun describe(info: NsdServiceInfo): FoundPc? {
        val address = addresses(info).sortedBy { if (it is Inet4Address) 0 else 1 }.firstOrNull() ?: return null
        val attributes = info.attributes.orEmpty()
        fun txt(key: String) = attributes[key]?.toString(Charsets.UTF_8)?.trim()?.takeIf { it.isNotEmpty() }
        return FoundPc(
            serviceName = info.serviceName,
            id = txt("id"),
            name = txt("name") ?: info.serviceName,
            host = address.hostAddress?.substringBefore('%') ?: return null,
            port = info.port,
        )
    }

    private fun addresses(info: NsdServiceInfo): List<InetAddress> =
        if (Build.VERSION.SDK_INT >= 34) {
            info.hostAddresses
        } else {
            @Suppress("DEPRECATION")
            listOfNotNull(info.host)
        }

    private companion object {
        const val RESOLVE_TIMEOUT_MS = 8_000L
    }
}
