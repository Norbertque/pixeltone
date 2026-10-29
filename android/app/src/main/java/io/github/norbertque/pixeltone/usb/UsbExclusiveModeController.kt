package io.github.norbertque.pixeltone.usb

import android.hardware.usb.UsbDeviceConnection
import io.github.norbertque.pixeltone.usbaudio.UsbAudioSession
import io.github.norbertque.pixeltone.usbaudio.descriptor.ParseResult
import io.github.norbertque.pixeltone.usbaudio.descriptor.UacCapabilities
import io.github.norbertque.pixeltone.usbaudio.descriptor.UacCapabilityProber
import io.github.norbertque.pixeltone.usbaudio.descriptor.UsbControlTransfer
import io.github.norbertque.pixeltone.usbaudio.descriptor.UsbDescriptorParser
import io.github.norbertque.pixeltone.usbaudio.negotiation.Conversion
import io.github.norbertque.pixeltone.usbaudio.negotiation.NegotiatedFormat
import io.github.norbertque.pixeltone.usbaudio.negotiation.SourceFormat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

/** Everything the UI (and the playback service) needs to know about USB exclusive mode. */
sealed interface UsbExclusiveState {
    /** The preference is off — feature entirely dormant, zero cost. */
    data object Disabled : UsbExclusiveState

    /** Enabled, but no USB Audio Class device attached. */
    data object NoDevice : UsbExclusiveState

    /** A DAC is attached; permission not yet requested/granted. */
    data class DeviceDetected(val device: UsbDeviceInfo) : UsbExclusiveState

    /** System permission dialog in flight. */
    data class PermissionPending(val device: UsbDeviceInfo) : UsbExclusiveState

    /** The user said no; a retry can be offered. */
    data class PermissionDenied(val device: UsbDeviceInfo) : UsbExclusiveState

    /** Device opened, capabilities probed, session established — the engine can attach. */
    data class Ready(val device: UsbDeviceInfo, val capabilities: UacCapabilities) : UsbExclusiveState

    /** Audio is flowing to the DAC. */
    data class Active(
        val device: UsbDeviceInfo,
        val capabilities: UacCapabilities,
        val format: NegotiatedFormat,
        val source: SourceFormat,
        val conversion: Conversion,
        val hardwareVolume: Boolean,
        /** Software volume in dB when the gain stage is engaged; null = unity path only. */
        val softwareVolumeDb: Float? = null
    ) : UsbExclusiveState {
        /** True bit-perfect: no format conversion AND no software attenuation. */
        val isBitPerfectOutput: Boolean
            get() = conversion.isBitPerfect && (softwareVolumeDb == null || softwareVolumeDb >= -0.01f)
    }

    /** Something failed; [recoverable] states can be retried from the UI. */
    data class Error(val message: String, val recoverable: Boolean) : UsbExclusiveState
}

/**
 * Owns the USB exclusive mode state machine and the [UsbAudioSession] lifecycle:
 * preference × attached devices × permission events in, session + [UsbExclusiveState] out.
 * The playback service moves the player onto the DAC (and back) in response to [state]; the
 * settings and player screens render it.
 */
class UsbExclusiveModeController(
    private val usbDeviceManager: UsbDeviceManager,
    private val settings: UsbSettings,
    private val scope: CoroutineScope,
    private val sessionFactory: UsbSessionFactory
) {
    private val _state = MutableStateFlow<UsbExclusiveState>(UsbExclusiveState.Disabled)
    val state: StateFlow<UsbExclusiveState> = _state.asStateFlow()

    /** Fires when an engaged session is lost involuntarily (unplug) — the service pauses. */
    private val _sessionLost = MutableSharedFlow<UsbDeviceInfo>(extraBufferCapacity = 4)
    val sessionLost: SharedFlow<UsbDeviceInfo> = _sessionLost.asSharedFlow()

    /** The established session while in Ready/Active; the engine builds its sink around it. */
    @Volatile
    var activeSession: UsbAudioSession? = null
        private set

    /** Software volume (dB) while the gain stage is engaged for the current session, else null. */
    private val _softwareVolumeDb = MutableStateFlow<Float?>(null)
    val softwareVolumeDb: StateFlow<Float?> = _softwareVolumeDb.asStateFlow()

    private val mutex = Mutex()
    private var sessionDevice: UsbDeviceInfo? = null
    private var deniedDeviceKeys = mutableSetOf<String>()
    private var requestedDeviceKeys = mutableSetOf<String>()
    private var fixedVolumeOutput = false
    private var maxVolumeAcknowledged = false
    /** Last known hardware volume fraction, cached so volume keys never block on USB I/O. */
    @Volatile
    private var cachedHardwareFraction: Float = 0.5f
    private var preMuteSoftwareDb: Float? = null

    init {
        scope.launch {
            combine(
                settings.exclusiveEnabled,
                usbDeviceManager.attachedAudioDevices
            ) { enabled, devices -> enabled to devices }
                .collect { (enabled, devices) -> reconcile(enabled, devices) }
        }
        scope.launch {
            usbDeviceManager.permissionEvents.collect { result -> onPermissionResult(result) }
        }
        scope.launch {
            settings.maxVolumeAcknowledged.collect { acknowledged ->
                maxVolumeAcknowledged = acknowledged
            }
        }
        scope.launch {
            settings.fixedVolumeOutput.collect { fixed ->
                fixedVolumeOutput = fixed
                // Live-apply to the running session: purist mode drops the gain stage.
                applySoftwareVolumeForCurrentSession()
            }
        }
    }

    /** Called from the settings UI to retry after a denial. */
    fun retryPermission(device: UsbDeviceInfo) {
        deniedDeviceKeys.remove(device.key)
        requestedDeviceKeys.remove(device.key)
        refresh()
    }

    /** Re-runs detection/reconciliation — the retry path for recoverable errors. */
    fun refresh() {
        usbDeviceManager.refreshAttachedDevices()
        scope.launch {
            reconcile(settings.exclusiveEnabled.value, usbDeviceManager.attachedAudioDevices.value)
        }
    }

    /** Sink → controller: the negotiated stream format changed (or ended with null). */
    fun onSinkFormatChanged(format: NegotiatedFormat?, source: SourceFormat?) {
        val device = sessionDevice ?: return
        val session = activeSession ?: return
        _state.value = if (format != null && source != null) {
            UsbExclusiveState.Active(
                device = device,
                capabilities = session.capabilities,
                format = format,
                source = source,
                conversion = format.conversion,
                hardwareVolume = session.capabilities.volume != null,
                softwareVolumeDb = _softwareVolumeDb.value
            )
        } else {
            UsbExclusiveState.Ready(device, session.capabilities)
        }
    }

    /** Sink → controller: writes started failing (device unplugged or driver died). */
    fun onSessionDead() {
        scope.launch {
            mutex.withLock {
                val device = sessionDevice ?: return@launch
                UsbLog.w(TAG, "Session died for %s", device.key)
                closeSessionLocked(lost = true)
                _state.value = UsbExclusiveState.NoDevice
            }
            usbDeviceManager.refreshAttachedDevices()
        }
    }

    /** Maps a 0..1 slider fraction onto the DAC's hardware volume range. */
    fun setHardwareVolume(fraction: Float): Boolean {
        val session = activeSession ?: return false
        val range = session.volumeRangeDb256() ?: return false
        val (min, max, resolution) = Triple(range[0], range[1], range[2].coerceAtLeast(1))
        val clamped = fraction.coerceIn(0f, 1f)
        val target = min + ((max - min) * clamped).toInt()
        val stepped = min + ((target - min) / resolution) * resolution
        return session.setVolumeDb256(stepped).also { ok ->
            if (ok) cachedHardwareFraction = clamped
        }
    }

    /** The DAC's current hardware volume as a 0..1 fraction of its range, or null. */
    fun hardwareVolumeFraction(): Float? {
        val session = activeSession ?: return null
        val range = session.volumeRangeDb256() ?: return null
        val current = session.currentVolumeDb256() ?: return null
        val (min, max) = range[0] to range[1]
        if (max <= min) return null
        return ((current - min).toFloat() / (max - min)).coerceIn(0f, 1f)
            .also { cachedHardwareFraction = it }
    }

    // ─── Software volume (DACs without a hardware feature unit) ────────────────

    /**
     * Sets the software volume, clamped to the allowed range — and, until the user has
     * acknowledged the loudness warning, additionally capped at
     * [UsbRememberedDevice.UNACKNOWLEDGED_CAP_DB]. Returns the dB actually applied,
     * or null when the gain stage is not in play (hardware volume / fixed output).
     */
    fun setSoftwareVolumeDb(db: Float): Float? {
        val session = activeSession ?: return null
        if (session.capabilities.volume != null || fixedVolumeOutput) return null

        val cap = if (maxVolumeAcknowledged) {
            UsbRememberedDevice.MAX_SOFTWARE_VOLUME_DB
        } else {
            UsbRememberedDevice.UNACKNOWLEDGED_CAP_DB
        }
        val applied = db.coerceIn(UsbRememberedDevice.MIN_SOFTWARE_VOLUME_DB, cap)
        preMuteSoftwareDb = null
        session.softwareGainQ16 = gainQ16ForDb(applied)
        _softwareVolumeDb.value = applied
        _state.update { state ->
            if (state is UsbExclusiveState.Active) state.copy(softwareVolumeDb = applied) else state
        }
        sessionDevice?.let { device ->
            scope.launch {
                val remembered = settings.rememberedDevices.value[device.key]
                settings.remember(
                    device.key,
                    (remembered ?: UsbRememberedDevice(label = device.displayName))
                        .copy(softwareVolumeDb = applied)
                )
            }
        }
        return applied
    }

    /** Installs (or removes) the gain stage for the current session per device/prefs. */
    private suspend fun applySoftwareVolumeForCurrentSession() {
        mutex.withLock { applySoftwareVolumeLocked() }
    }

    private suspend fun applySoftwareVolumeLocked() {
        run {
            val session = activeSession ?: return
            val device = sessionDevice ?: return
            if (session.capabilities.volume != null || fixedVolumeOutput) {
                session.softwareGainQ16 = UsbAudioSession.UNITY_GAIN_Q16
                _softwareVolumeDb.value = null
                _state.update { state ->
                    if (state is UsbExclusiveState.Active) state.copy(softwareVolumeDb = null) else state
                }
                return
            }
            val rememberedDb = settings.rememberedDevices.value[device.key]
                ?.softwareVolumeDb ?: UsbRememberedDevice.DEFAULT_SOFTWARE_VOLUME_DB
            val cap = if (maxVolumeAcknowledged) {
                UsbRememberedDevice.MAX_SOFTWARE_VOLUME_DB
            } else {
                UsbRememberedDevice.UNACKNOWLEDGED_CAP_DB
            }
            val applied = rememberedDb.coerceIn(UsbRememberedDevice.MIN_SOFTWARE_VOLUME_DB, cap)
            session.softwareGainQ16 = gainQ16ForDb(applied)
            _softwareVolumeDb.value = applied
            _state.update { state ->
                if (state is UsbExclusiveState.Active) state.copy(softwareVolumeDb = applied) else state
            }
            UsbLog.i(TAG, "Software volume engaged at %.0f dB for %s", applied, device.key)
        }
    }


    // ─── Device-volume mapping (phone volume keys via the media session) ──────

    /** Total steps advertised to the media session's device-volume surface. */
    val deviceVolumeMaxSteps: Int get() = DEVICE_VOLUME_STEPS

    /** True when volume keys have something to control (hw feature unit or software gain). */
    fun deviceVolumeAvailable(): Boolean {
        val session = activeSession ?: return false
        return session.capabilities.volume != null ||
            (!fixedVolumeOutput && _softwareVolumeDb.value != null)
    }

    fun deviceVolumeSteps(): Int {
        val session = activeSession ?: return 0
        return if (session.capabilities.volume != null) {
            (cachedHardwareFraction * DEVICE_VOLUME_STEPS).toInt().coerceIn(0, DEVICE_VOLUME_STEPS)
        } else {
            val db = _softwareVolumeDb.value ?: return 0
            stepsForDb(db)
        }
    }

    fun setDeviceVolumeSteps(steps: Int) {
        val session = activeSession ?: return
        val clamped = steps.coerceIn(0, DEVICE_VOLUME_STEPS)
        if (session.capabilities.volume != null) {
            scope.launch(Dispatchers.IO) {
                setHardwareVolume(clamped.toFloat() / DEVICE_VOLUME_STEPS)
            }
        } else {
            setSoftwareVolumeDb(dbForSteps(clamped))
        }
    }

    fun adjustDeviceVolume(delta: Int) = setDeviceVolumeSteps(deviceVolumeSteps() + delta)

    fun setDeviceMuted(muted: Boolean) {
        val session = activeSession ?: return
        if (session.capabilities.volume != null) {
            scope.launch(Dispatchers.IO) { session.setMute(muted) }
        } else if (muted) {
            preMuteSoftwareDb = _softwareVolumeDb.value
            setSoftwareVolumeDb(UsbRememberedDevice.MIN_SOFTWARE_VOLUME_DB)
        } else {
            setSoftwareVolumeDb(preMuteSoftwareDb ?: UsbRememberedDevice.DEFAULT_SOFTWARE_VOLUME_DB)
        }
    }

    private fun gainQ16ForDb(db: Float): Int = when {
        db >= -0.01f -> UsbAudioSession.UNITY_GAIN_Q16
        db <= UsbRememberedDevice.MIN_SOFTWARE_VOLUME_DB -> 0 // floor = mute
        else -> (Math.pow(10.0, db / 20.0) * UsbAudioSession.UNITY_GAIN_Q16).toInt()
    }

    private fun stepsForDb(db: Float): Int =
        (((db - UsbRememberedDevice.MIN_SOFTWARE_VOLUME_DB) /
            (UsbRememberedDevice.MAX_SOFTWARE_VOLUME_DB - UsbRememberedDevice.MIN_SOFTWARE_VOLUME_DB)) *
            DEVICE_VOLUME_STEPS).toInt().coerceIn(0, DEVICE_VOLUME_STEPS)

    private fun dbForSteps(steps: Int): Float =
        UsbRememberedDevice.MIN_SOFTWARE_VOLUME_DB +
            (UsbRememberedDevice.MAX_SOFTWARE_VOLUME_DB - UsbRememberedDevice.MIN_SOFTWARE_VOLUME_DB) *
            steps.toFloat() / DEVICE_VOLUME_STEPS

    private suspend fun reconcile(enabled: Boolean, devices: List<UsbDeviceInfo>) {
        mutex.withLock {
            if (!enabled) {
                closeSessionLocked(lost = false)
                deniedDeviceKeys.clear()
                requestedDeviceKeys.clear()
                _state.value = UsbExclusiveState.Disabled
                return
            }

            // Keep the current session while its device stays attached.
            val currentDevice = sessionDevice
            if (currentDevice != null) {
                val stillAttached = devices.any { it.deviceName == currentDevice.deviceName }
                val stillAlive = activeSession?.isAlive != false
                if (stillAttached && stillAlive) return
                closeSessionLocked(lost = true)
            }

            val device = devices.firstOrNull()
            if (device == null) {
                _state.value = UsbExclusiveState.NoDevice
                return
            }

            when {
                device.hasPermission -> openSessionLocked(device)
                device.key in deniedDeviceKeys ->
                    _state.value = UsbExclusiveState.PermissionDenied(device)
                device.key in requestedDeviceKeys ->
                    _state.value = UsbExclusiveState.PermissionPending(device)
                else -> {
                    // The user opted into exclusive mode, so ask right away on attach.
                    requestedDeviceKeys.add(device.key)
                    _state.value = UsbExclusiveState.PermissionPending(device)
                    usbDeviceManager.requestPermission(device)
                }
            }
        }
    }

    private suspend fun onPermissionResult(result: UsbPermissionResult) {
        when (result) {
            is UsbPermissionResult.Granted -> {
                deniedDeviceKeys.remove(result.device.key)
                // Keep what was remembered of it (its volume); only its name may have changed.
                val remembered = settings.rememberedDevices.value[result.device.key]
                settings.remember(
                    result.device.key,
                    remembered?.copy(label = result.device.displayName)
                        ?: UsbRememberedDevice(label = result.device.displayName)
                )
                mutex.withLock {
                    if (sessionDevice == null && settings.exclusiveEnabled.value) {
                        openSessionLocked(result.device)
                    }
                }
            }

            is UsbPermissionResult.Denied -> {
                deniedDeviceKeys.add(result.device.key)
                if (sessionDevice == null) {
                    _state.value = UsbExclusiveState.PermissionDenied(result.device)
                }
            }
        }
    }

    private suspend fun openSessionLocked(device: UsbDeviceInfo) {
        val opened = withContext(Dispatchers.IO) {
            val connection = usbDeviceManager.openConnection(device)
                ?: return@withContext OpenOutcome.Failed("Could not open ${device.displayName}", true)

            val raw = connection.rawDescriptors
                ?: return@withContext OpenOutcome.Failed("No descriptors from ${device.displayName}", true)
                    .also { connection.close() }

            val topology = when (val parsed = UsbDescriptorParser.parse(raw)) {
                is ParseResult.Failure -> {
                    UsbLog.w(TAG, "Descriptor parse failed for %s: %s", device.key, parsed.reason)
                    logRawDescriptors(raw)
                    connection.close()
                    return@withContext OpenOutcome.Failed(
                        "${device.displayName}: ${parsed.reason}", false
                    )
                }
                is ParseResult.Success -> parsed.topology
            }
            UsbLog.i(
                TAG,
                "Topology %s: %s, AC=%d, altSettings=%s, clockSources=%s, selectors=%s",
                device.key, topology.version, topology.controlInterfaceNumber,
                topology.playbackAltSettings.map {
                    "if${it.interfaceNumber}/alt${it.altSetting} ${it.bitResolution}bit×${it.channels}ch ${it.dataEndpoint.syncType}"
                },
                topology.clockSources.map { it.id },
                topology.clockSelectors.map { "${it.id}(pins=${it.pinSourceIds})" }
            )

            // The session (and its claim) must exist BEFORE rate probing: usbfs rejects
            // interface-recipient control transfers while the kernel audio driver still
            // owns the interface, so probing goes through libusb after the detach+claim.
            val session = sessionFactory.open(connection, UacCapabilityProber.preliminary(topology))
                ?: return@withContext OpenOutcome.Failed(
                    "Driver could not attach to ${device.displayName}", true
                ).also { connection.close() }

            val asInterface = topology.playbackAltSettings.first().interfaceNumber
            if (!session.claim(topology.controlInterfaceNumber, asInterface)) {
                val reason = session.lastError ?: "claim failed"
                session.close()
                return@withContext OpenOutcome.Failed(
                    "Could not claim ${device.displayName}: $reason", true
                )
            }

            val controlTransfer = UsbControlTransfer { requestType, request, value, index, buffer ->
                session.controlTransferIn(requestType, request, value, index, buffer)
            }
            val capabilities = UacCapabilityProber.probe(topology, controlTransfer)
            if (capabilities.formats.isEmpty()) {
                UsbLog.w(
                    TAG,
                    "No formats resolved for %s (driver error: %s)", device.key, session.lastError
                )
                logRawDescriptors(raw)
                session.close()
                return@withContext OpenOutcome.Failed(
                    "Could not read supported formats from ${device.displayName}", true
                )
            }
            session.installCapabilities(capabilities)

            OpenOutcome.Opened(session, capabilities)
        }

        when (opened) {
            is OpenOutcome.Opened -> {
                activeSession = opened.session
                sessionDevice = device
                _state.value = UsbExclusiveState.Ready(device, opened.capabilities)
                if (opened.capabilities.volume != null) {
                    // Prime the cache so volume keys have a truthful starting point.
                    hardwareVolumeFraction()
                } else {
                    // Volume-less DAC: engage the software gain stage at the remembered
                    // (safe-by-default) level before any audio can flow.
                    applySoftwareVolumeLocked()
                }
                UsbLog.i(
                    TAG,
                    "USB exclusive ready: %s (%s, rates=%s, swVolume=%s)",
                    device.displayName, opened.capabilities.version,
                    opened.capabilities.allSampleRatesHz, _softwareVolumeDb.value
                )
            }

            is OpenOutcome.Failed -> {
                UsbLog.w(TAG, "USB exclusive open failed: %s", opened.message)
                _state.value = UsbExclusiveState.Error(opened.message, opened.recoverable)
            }
        }
    }

    private suspend fun closeSessionLocked(lost: Boolean) {
        val session = activeSession ?: return
        val device = sessionDevice
        activeSession = null
        sessionDevice = null
        _softwareVolumeDb.value = null
        preMuteSoftwareDb = null
        withContext(Dispatchers.IO) { runCatching { session.close() } }
        if (lost && device != null) {
            _sessionLost.tryEmit(device)
        }
        UsbLog.i(TAG, "USB session closed (lost=%b)", lost)
    }

    /** Field-debugging aid: the raw descriptor blob, hex-dumped in logcat-sized lines. */
    private fun logRawDescriptors(raw: ByteArray) {
        raw.toList().chunked(64).forEachIndexed { index, chunk ->
            UsbLog.d(
                TAG,
                "descriptors[%03d]: %s", index * 64,
                chunk.joinToString("") { "%02x".format(it) }
            )
        }
    }

    private sealed interface OpenOutcome {
        data class Opened(val session: UsbAudioSession, val capabilities: UacCapabilities) : OpenOutcome
        data class Failed(val message: String, val recoverable: Boolean) : OpenOutcome
    }

    private companion object {
        private const val TAG = "UsbExclusiveMode"
        private const val DEVICE_VOLUME_STEPS = 30
    }
}

/** Indirection over [UsbAudioSession.open] so the state machine is unit-testable. */
fun interface UsbSessionFactory {
    fun open(connection: UsbDeviceConnection, capabilities: UacCapabilities): UsbAudioSession?
}
