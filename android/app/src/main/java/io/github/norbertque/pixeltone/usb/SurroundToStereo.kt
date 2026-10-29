package io.github.norbertque.pixeltone.usb

import androidx.media3.common.audio.ChannelMixingAudioProcessor
import androidx.media3.common.audio.ChannelMixingMatrix
import androidx.media3.common.util.UnstableApi

/**
 * 5.1 and 7.1 folded into stereo for a two-channel DAC: centre and surrounds at −3 dB, the LFE
 * left out (ITU-R BS.775), scaled so that full scale on every channel still fits. Only ever on
 * the converted path; the badge says so.
 */
@UnstableApi
internal fun surroundToStereo(): ChannelMixingAudioProcessor =
    ChannelMixingAudioProcessor().apply {
        putChannelMixingMatrix(toStereo(6))
        putChannelMixingMatrix(toStereo(8))
    }

private const val MINUS_3_DB = 0.70710677f

/** Rows are the input channels in Android's order (FL FR FC LFE BL BR, then SL SR); columns L R. */
@UnstableApi
private fun toStereo(channels: Int): ChannelMixingMatrix {
    val coefficients = mutableListOf(
        1f, 0f, // FL
        0f, 1f, // FR
        MINUS_3_DB, MINUS_3_DB, // FC
        0f, 0f, // LFE
        MINUS_3_DB, 0f, // BL
        0f, MINUS_3_DB, // BR
    )
    if (channels == 8) coefficients += listOf(MINUS_3_DB, 0f, 0f, MINUS_3_DB) // SL, SR
    val loudest = coefficients.filterIndexed { i, _ -> i % 2 == 0 }.sum()
    return ChannelMixingMatrix(channels, 2, coefficients.toFloatArray()).scaleBy(1f / loudest)
}
