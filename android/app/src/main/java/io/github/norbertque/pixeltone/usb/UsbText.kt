package io.github.norbertque.pixeltone.usb

import io.github.norbertque.pixeltone.usbaudio.descriptor.UacCapabilities
import java.util.Locale

/** "44.1 kHz", "96 kHz", "352.8 kHz". */
fun formatRate(hz: Int): String =
    if (hz % 1000 == 0) "${hz / 1000} kHz" else String.format(Locale.ROOT, "%.1f kHz", hz / 1000.0)

/** "24-bit/96 kHz". */
fun formatPcm(bits: Int, hz: Int): String = "$bits-bit/${formatRate(hz)}"

/** The most a DAC takes: "32-bit/384 kHz". */
fun bestFormat(capabilities: UacCapabilities): String =
    formatPcm(capabilities.allBitResolutions.maxOrNull() ?: 16, capabilities.allSampleRatesHz.maxOrNull() ?: 48_000)

/** "−30 dB", with a real minus. */
fun formatDb(db: Float): String = String.format(Locale.ROOT, "%.0f dB", db).replace('-', '−')

/**
 * What goes into the DAC, as Now Playing shows it: "USB • 24-bit/96 kHz • Bit-perfect", or
 * "Converted" when the DAC can't take the source as it is, or the software volume when it's
 * turned down (bit-perfect only at 0 dB). Null unless audio is flowing to a DAC.
 */
fun usbBadge(state: UsbExclusiveState): String? {
    val active = state as? UsbExclusiveState.Active ?: return null
    val parts = mutableListOf("USB", formatPcm(active.format.candidate.bitResolution, active.format.sampleRateHz))
    val attenuation = active.softwareVolumeDb?.takeIf { it < -0.01f }
    when {
        !active.conversion.isBitPerfect -> parts += "Converted"
        attenuation != null -> parts += formatDb(attenuation)
        else -> parts += "Bit-perfect"
    }
    return parts.joinToString(" • ")
}
