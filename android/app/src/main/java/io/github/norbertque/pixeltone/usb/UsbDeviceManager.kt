package io.github.norbertque.pixeltone.usb

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbConstants
import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbDeviceConnection
import android.hardware.usb.UsbManager
import android.os.Build
import androidx.core.content.ContextCompat
import androidx.core.content.IntentCompat
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * Tracks attached USB Audio Class devices and owns the USB permission flow.
 *
 * Detection is broadcast-driven (ATTACHED/DETACHED) plus an enumeration at construction time so
 * a DAC that was already plugged in when the app started is found too. Only devices exposing an
 * AudioStreaming interface (class 0x01 / subclass 0x02) are listed.
 */
class UsbDeviceManager(private val context: Context) {
    private val usbManager: UsbManager? =
        context.getSystemService(Context.USB_SERVICE) as? UsbManager

    /** The app's own (release and debug builds each have theirs). */
    private val permissionAction = "${context.packageName}.USB_PERMISSION"

    private val _attachedAudioDevices = MutableStateFlow<List<UsbDeviceInfo>>(emptyList())
    val attachedAudioDevices: StateFlow<List<UsbDeviceInfo>> = _attachedAudioDevices.asStateFlow()

    private val _permissionEvents = MutableSharedFlow<UsbPermissionResult>(extraBufferCapacity = 8)
    val permissionEvents: SharedFlow<UsbPermissionResult> = _permissionEvents.asSharedFlow()

    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            when (intent.action) {
                UsbManager.ACTION_USB_DEVICE_ATTACHED,
                UsbManager.ACTION_USB_DEVICE_DETACHED -> refreshAttachedDevices()

                permissionAction -> {
                    val device: UsbDevice? =
                        IntentCompat.getParcelableExtra(intent, UsbManager.EXTRA_DEVICE, UsbDevice::class.java)
                    val granted = intent.getBooleanExtra(UsbManager.EXTRA_PERMISSION_GRANTED, false)
                    // Refresh first so the emitted snapshot carries hasPermission/serial info.
                    refreshAttachedDevices()
                    if (device != null) {
                        val info = device.toInfo()
                        UsbLog.i(TAG, "USB permission %s for %s", if (granted) "granted" else "denied", info.key)
                        _permissionEvents.tryEmit(
                            if (granted) UsbPermissionResult.Granted(info)
                            else UsbPermissionResult.Denied(info)
                        )
                    }
                }
            }
        }
    }

    init {
        if (usbManager != null) {
            val filter = IntentFilter().apply {
                addAction(UsbManager.ACTION_USB_DEVICE_ATTACHED)
                addAction(UsbManager.ACTION_USB_DEVICE_DETACHED)
                addAction(permissionAction)
            }
            // NOT_EXPORTED still receives system broadcasts; the permission action is only ever
            // sent by our own PendingIntent (explicit package below).
            ContextCompat.registerReceiver(context, receiver, filter, ContextCompat.RECEIVER_NOT_EXPORTED)
            refreshAttachedDevices()
        } else {
            UsbLog.w(TAG, "UsbManager unavailable; USB exclusive mode disabled")
        }
    }

    /** Shows the system USB permission dialog; result arrives on [permissionEvents]. */
    fun requestPermission(device: UsbDeviceInfo) {
        val manager = usbManager ?: return
        val raw = findRawDevice(device) ?: run {
            UsbLog.w(TAG, "requestPermission: %s no longer attached", device.key)
            _permissionEvents.tryEmit(UsbPermissionResult.Denied(device))
            return
        }
        if (manager.hasPermission(raw)) {
            _permissionEvents.tryEmit(UsbPermissionResult.Granted(raw.toInfo()))
            return
        }
        val intent = Intent(permissionAction).setPackage(context.packageName)
        // Mutable: the system fills in EXTRA_DEVICE/EXTRA_PERMISSION_GRANTED (and before
        // Android 12 every PendingIntent was).
        val mutable = if (Build.VERSION.SDK_INT >= 31) PendingIntent.FLAG_MUTABLE else 0
        val pendingIntent = PendingIntent.getBroadcast(
            context,
            device.key.hashCode(),
            intent,
            mutable or PendingIntent.FLAG_UPDATE_CURRENT
        )
        manager.requestPermission(raw, pendingIntent)
    }

    fun hasPermission(device: UsbDeviceInfo): Boolean {
        val manager = usbManager ?: return false
        val raw = findRawDevice(device) ?: return false
        return manager.hasPermission(raw)
    }

    /**
     * Opens the device, yielding the file descriptor holder the native driver wraps.
     * Returns null when the device is gone or permission is missing.
     */
    fun openConnection(device: UsbDeviceInfo): UsbDeviceConnection? {
        val manager = usbManager ?: return null
        val raw = findRawDevice(device) ?: return null
        if (!manager.hasPermission(raw)) return null
        return manager.openDevice(raw)
    }

    fun refreshAttachedDevices() {
        val manager = usbManager ?: return
        val devices = manager.deviceList.values
            .filter(::hasAudioStreamingInterface)
            .map { it.toInfo() }
            .sortedBy { it.deviceName }
        _attachedAudioDevices.value = devices
    }

    private fun findRawDevice(device: UsbDeviceInfo): UsbDevice? =
        usbManager?.deviceList?.values?.firstOrNull { it.deviceName == device.deviceName }

    private fun UsbDevice.toInfo(): UsbDeviceInfo {
        val permitted = usbManager?.hasPermission(this) == true
        return UsbDeviceInfo(
            deviceName = deviceName,
            productName = productName,
            manufacturerName = manufacturerName,
            vendorId = vendorId,
            productId = productId,
            // Reading the serial without permission throws on API 29+.
            serialNumber = if (permitted) runCatching { serialNumber }.getOrNull() else null,
            hasPermission = permitted
        )
    }

    companion object {
        private const val TAG = "UsbDeviceManager"
    }
}

/**
 * True when the device exposes a USB Audio Class AudioStreaming interface — i.e. it can carry
 * an audio stream, as opposed to audio-class control-only or HID/storage composites.
 */
internal fun hasAudioStreamingInterface(device: UsbDevice): Boolean {
    for (i in 0 until device.interfaceCount) {
        val itf = device.getInterface(i)
        if (itf.interfaceClass == UsbConstants.USB_CLASS_AUDIO &&
            itf.interfaceSubclass == USB_SUBCLASS_AUDIOSTREAMING
        ) {
            return true
        }
    }
    return false
}

/** USB Audio Class interface subclass code for AudioStreaming (UAC1 §A.2 / UAC2 §A.5). */
internal const val USB_SUBCLASS_AUDIOSTREAMING = 0x02
