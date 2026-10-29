package io.github.norbertque.pixeltone.usb

import android.content.Context
import androidx.core.content.edit
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.builtins.MapSerializer
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.json.Json

/** USB output's settings: the switch, the loudness acknowledgement, and each DAC remembered. */
interface UsbSettings {
    /** Exclusive USB output: Pixeltone takes over a USB DAC and plays bit-perfect. */
    val exclusiveEnabled: StateFlow<Boolean>

    /** The user read the loudness warning: software volume may go above its cap. */
    val maxVolumeAcknowledged: StateFlow<Boolean>

    /** Full scale, no software volume: for an amplifier after the DAC. */
    val fixedVolumeOutput: StateFlow<Boolean>

    /** By [UsbDeviceInfo.key]. */
    val rememberedDevices: StateFlow<Map<String, UsbRememberedDevice>>

    fun setExclusiveEnabled(on: Boolean)

    fun setMaxVolumeAcknowledged(on: Boolean)

    fun setFixedVolumeOutput(on: Boolean)

    fun remember(key: String, device: UsbRememberedDevice)
}

/** [UsbSettings] in the app's preferences. */
class PreferencesUsbSettings(context: Context) : UsbSettings {
    private val prefs = context.getSharedPreferences("usb", Context.MODE_PRIVATE)
    private val json = Json { ignoreUnknownKeys = true }
    private val devicesSerializer = MapSerializer(String.serializer(), UsbRememberedDevice.serializer())

    private val enabled = MutableStateFlow(prefs.getBoolean(ENABLED, false))
    private val acknowledged = MutableStateFlow(prefs.getBoolean(ACKNOWLEDGED, false))
    private val fixed = MutableStateFlow(prefs.getBoolean(FIXED, false))
    private val devices = MutableStateFlow(
        prefs.getString(DEVICES, null)
            ?.let { runCatching { json.decodeFromString(devicesSerializer, it) }.getOrNull() }
            .orEmpty(),
    )

    override val exclusiveEnabled: StateFlow<Boolean> = enabled.asStateFlow()
    override val maxVolumeAcknowledged: StateFlow<Boolean> = acknowledged.asStateFlow()
    override val fixedVolumeOutput: StateFlow<Boolean> = fixed.asStateFlow()
    override val rememberedDevices: StateFlow<Map<String, UsbRememberedDevice>> = devices.asStateFlow()

    override fun setExclusiveEnabled(on: Boolean) = save(ENABLED, on, enabled)

    override fun setMaxVolumeAcknowledged(on: Boolean) = save(ACKNOWLEDGED, on, acknowledged)

    override fun setFixedVolumeOutput(on: Boolean) = save(FIXED, on, fixed)

    @Synchronized
    override fun remember(key: String, device: UsbRememberedDevice) {
        val updated = devices.value + (key to device)
        prefs.edit { putString(DEVICES, json.encodeToString(devicesSerializer, updated)) }
        devices.value = updated
    }

    private fun save(key: String, on: Boolean, flow: MutableStateFlow<Boolean>) {
        prefs.edit { putBoolean(key, on) }
        flow.value = on
    }

    private companion object {
        const val ENABLED = "exclusive"
        const val ACKNOWLEDGED = "loudness_acknowledged"
        const val FIXED = "fixed_volume"
        const val DEVICES = "devices"
    }
}
