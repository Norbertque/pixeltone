package io.github.norbertque.pixeltone.usb

import io.github.norbertque.pixeltone.usbaudio.descriptor.EndpointSyncType
import io.github.norbertque.pixeltone.usbaudio.descriptor.FormatCandidate
import io.github.norbertque.pixeltone.usbaudio.descriptor.UacCapabilities
import io.github.norbertque.pixeltone.usbaudio.descriptor.UacVersion
import io.github.norbertque.pixeltone.usbaudio.negotiation.Conversion
import io.github.norbertque.pixeltone.usbaudio.negotiation.NegotiatedFormat
import io.github.norbertque.pixeltone.usbaudio.negotiation.SourceFormat
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class UsbTextTest {
    @Test
    fun `rates and formats read like the PC's badge`() {
        assertEquals("44.1 kHz", formatRate(44_100))
        assertEquals("96 kHz", formatRate(96_000))
        assertEquals("352.8 kHz", formatRate(352_800))
        assertEquals("24-bit/96 kHz", formatPcm(24, 96_000))
        assertEquals("32-bit/384 kHz", bestFormat(SAMPLE_DAC))
        assertEquals("−30 dB", formatDb(-30f))
    }

    @Test
    fun `the badge says bit-perfect only for untouched output`() {
        val untouched = active(Conversion.BIT_PERFECT, softwareDb = null)
        assertEquals("USB • 24-bit/96 kHz • Bit-perfect", usbBadge(untouched))
        assertEquals("USB • 24-bit/96 kHz • Bit-perfect", usbBadge(active(Conversion.BIT_PERFECT, softwareDb = 0f)))
        assertEquals("USB • 24-bit/96 kHz • −30 dB", usbBadge(active(Conversion.BIT_PERFECT, softwareDb = -30f)))
        val resampled = Conversion(resampled = true, depthReduced = false, downmixed = false)
        assertEquals("USB • 24-bit/96 kHz • Converted", usbBadge(active(resampled, softwareDb = null)))
        assertNull(usbBadge(UsbExclusiveState.NoDevice))
        assertNull(usbBadge(UsbExclusiveState.Ready(DAC, SAMPLE_DAC)))
    }

    private fun active(conversion: Conversion, softwareDb: Float?) = UsbExclusiveState.Active(
        device = DAC,
        capabilities = SAMPLE_DAC,
        format = NegotiatedFormat(SAMPLE_DAC.formats[1], 96_000, conversion),
        source = SourceFormat(96_000, 24, 2, isFloat = false),
        conversion = conversion,
        hardwareVolume = false,
        softwareVolumeDb = softwareDb,
    )
}

internal val DAC = UsbDeviceInfo("/dev/bus/usb/001/002", "Apogee Groove", "Apogee", 0x0c60, 0x0014, "S1", hasPermission = true)

internal fun candidate(altSetting: Int, bits: Int, subslot: Int, rates: List<Int>) = FormatCandidate(
    interfaceNumber = 1,
    altSetting = altSetting,
    channels = 2,
    subslotBytes = subslot,
    bitResolution = bits,
    sampleRatesHz = rates,
    endpointAddress = 0x01,
    maxPacketSize = 1024,
    intervalCode = 1,
    syncType = EndpointSyncType.ASYNCHRONOUS,
    feedbackEndpointAddress = 0x81,
    clockSourceId = 41,
    uac1SampleRateControl = false,
)

internal val SAMPLE_DAC = UacCapabilities(
    version = UacVersion.UAC2,
    controlInterfaceNumber = 0,
    formats = listOf(
        candidate(1, 16, 2, listOf(44_100, 48_000, 88_200, 96_000)),
        candidate(2, 24, 3, listOf(44_100, 48_000, 88_200, 96_000, 176_400, 192_000)),
        candidate(3, 32, 4, listOf(44_100, 48_000, 88_200, 96_000, 176_400, 192_000, 352_800, 384_000)),
    ),
    volume = null,
)
