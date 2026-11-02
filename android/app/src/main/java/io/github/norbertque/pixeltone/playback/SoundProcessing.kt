package io.github.norbertque.pixeltone.playback

import androidx.annotation.OptIn
import androidx.media3.common.C
import androidx.media3.common.Format
import androidx.media3.common.Metadata
import androidx.media3.common.audio.AudioProcessor.AudioFormat
import androidx.media3.common.audio.AudioProcessor.UnhandledAudioFormatException
import androidx.media3.common.audio.BaseAudioProcessor
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.audio.AudioSink
import androidx.media3.exoplayer.audio.ForwardingAudioSink
import androidx.media3.extractor.metadata.id3.InternalFrame
import androidx.media3.extractor.metadata.id3.TextInformationFrame
import androidx.media3.extractor.metadata.vorbis.VorbisComment
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.Locale
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.log10
import kotlin.math.pow
import kotlin.math.roundToInt
import kotlin.math.sin

/** A track's ReplayGain tags (gains in dB, peaks as fractions of full scale). */
data class ReplayGainTags(
    val trackGainDb: Float? = null,
    val trackPeak: Float? = null,
    val albumGainDb: Float? = null,
    val albumPeak: Float? = null,
)

/**
 * The gain ReplayGain asks for, as on Windows (`replay_gain_db`): the chosen value (else the
 * other one) plus [preampDb], never more than the tagged peak allows; null when off or untagged.
 */
fun replayGainDb(mode: ReplayGainMode, preampDb: Float, tags: ReplayGainTags): Float? {
    val (gain, peak) = when (mode) {
        ReplayGainMode.Off -> return null
        ReplayGainMode.Track -> (tags.trackGainDb ?: tags.albumGainDb) to (tags.trackPeak ?: tags.albumPeak)
        ReplayGainMode.Album -> (tags.albumGainDb ?: tags.trackGainDb) to (tags.albumPeak ?: tags.trackPeak)
    }
    var db = (gain ?: return null) + preampDb
    if (peak != null && peak > 0f) db = minOf(db, -20f * log10(peak))
    return db.takeIf { abs(it) >= 0.01f }
}

/**
 * ReplayGain tags among a track's metadata: Vorbis comments (FLAC, Ogg Vorbis, Opus), ID3 TXXX
 * frames (MP3) and iTunes' own entries (M4A). Opus's R128 gains count too.
 */
@OptIn(UnstableApi::class)
fun replayGainTagsOf(metadata: Metadata?): ReplayGainTags {
    if (metadata == null) return ReplayGainTags()
    val values = HashMap<String, String>()
    for (i in 0 until metadata.length()) {
        when (val entry = metadata[i]) {
            is VorbisComment -> values.putIfAbsent(entry.key.uppercase(Locale.ROOT), entry.value)
            is TextInformationFrame -> if (entry.id == "TXXX") {
                entry.description?.let { values.putIfAbsent(it.uppercase(Locale.ROOT), entry.values.firstOrNull().orEmpty()) }
            }
            is InternalFrame -> values.putIfAbsent(entry.description.uppercase(Locale.ROOT), entry.text)
        }
    }
    fun number(key: String) = values[key]?.let(::leadingNumber)
    // R128 gains are Q7.8 dB towards −23 LUFS; ReplayGain's −18 LUFS is 5 dB louder.
    fun r128(key: String) = values[key]?.trim()?.toIntOrNull()?.let { it / 256f + 5f }
    return ReplayGainTags(
        trackGainDb = number("REPLAYGAIN_TRACK_GAIN") ?: r128("R128_TRACK_GAIN"),
        trackPeak = number("REPLAYGAIN_TRACK_PEAK"),
        albumGainDb = number("REPLAYGAIN_ALBUM_GAIN") ?: r128("R128_ALBUM_GAIN"),
        albumPeak = number("REPLAYGAIN_ALBUM_PEAK"),
    )
}

private val LEADING_NUMBER = Regex("""^\s*([+-]?(?:\d+(?:\.\d*)?|\.\d+))""")

/** "-6.50 dB" → −6.5, as Windows reads its tags. */
internal fun leadingNumber(text: String): Float? = LEADING_NUMBER.find(text)?.groupValues?.get(1)?.toFloatOrNull()

/**
 * A 10-band graphic equaliser, as on Windows (`pp-core` eq.rs): octave-wide RBJ peaking
 * biquads in double, one for each band that isn't flat, after a pre-amp that leaves room for
 * the boosts. New gains keep each band's state, so changes don't click.
 */
class Equalizer(val sampleRate: Int, val channels: Int) {
    private class Section(val band: Int, val b0: Double, val b1: Double, val b2: Double, val a1: Double, val a2: Double)

    private var sections = emptyList<Section>()

    /** Transposed direct form II state (z1, z2) per section per channel. */
    private var state = DoubleArray(0)
    private var preamp = 1.0

    fun set(settings: EqSettings) {
        val rate = sampleRate.toDouble()
        val next = EQ_BANDS.indices.mapNotNull { band ->
            val gain = settings.gainsDb.getOrElse(band) { 0f }
            if (abs(gain) < 0.05f || EQ_BANDS[band] >= rate * 0.45) return@mapNotNull null
            peaking(band, rate, EQ_BANDS[band].toDouble(), gain.coerceIn(-EQ_MAX_DB, EQ_MAX_DB).toDouble())
        }
        val width = channels.coerceAtLeast(1)
        val nextState = DoubleArray(next.size * width * 2)
        next.forEachIndexed { s, section ->
            val old = sections.indexOfFirst { it.band == section.band }
            if (old >= 0) state.copyInto(nextState, s * width * 2, old * width * 2, (old + 1) * width * 2)
        }
        sections = next
        state = nextState
        preamp = 10.0.pow(settings.preampDb / 20.0)
    }

    /** Filters [count] interleaved samples of [samples] in place. */
    fun process(samples: DoubleArray, count: Int) {
        val width = channels.coerceAtLeast(1)
        for (i in 0 until count) {
            val channel = i % width
            var v = samples[i] * preamp
            for ((s, c) in sections.withIndex()) {
                val z = (s * width + channel) * 2
                val y = c.b0 * v + state[z]
                state[z] = c.b1 * v - c.a1 * y + state[z + 1]
                state[z + 1] = c.b2 * v - c.a2 * y
                v = y
            }
            samples[i] = v
        }
    }

    private companion object {
        /** About an octave wide. */
        const val Q = 1.41

        fun peaking(band: Int, rate: Double, freq: Double, gainDb: Double): Section {
            val a = 10.0.pow(gainDb / 40)
            val w0 = 2 * PI * freq / rate
            val alpha = sin(w0) / (2 * Q)
            val cos = cos(w0)
            val a0 = 1 + alpha / a
            return Section(band, (1 + alpha * a) / a0, -2 * cos / a0, (1 - alpha * a) / a0, -2 * cos / a0, (1 - alpha / a) / a0)
        }
    }
}

/**
 * Pixeltone's sound processing on Android's output, as on Windows: the equaliser, then
 * ReplayGain. With both off, the samples pass untouched. A track's ReplayGain tags take over
 * at its first sample: [SoundSink] hands them over with the track's format, and the sink flushes
 * its processors where the new format begins.
 */
@OptIn(UnstableApi::class)
class SoundProcessor(
    private val sound: () -> Sound,
    private val onTrackTags: (ReplayGainTags) -> Unit = {},
) : BaseAudioProcessor() {
    /** The tags of the track the coming configuration is for. */
    @Volatile
    var nextTags = ReplayGainTags()

    private var tags = ReplayGainTags()
    private var equalizer: Equalizer? = null
    private var equalizerFor: EqSettings? = null
    private var samples = DoubleArray(0)

    override fun onConfigure(inputAudioFormat: AudioFormat): AudioFormat {
        if (inputAudioFormat.encoding != C.ENCODING_PCM_16BIT && inputAudioFormat.encoding != C.ENCODING_PCM_FLOAT) {
            throw UnhandledAudioFormatException(inputAudioFormat)
        }
        return inputAudioFormat
    }

    override fun onFlush() {
        tags = nextTags
        onTrackTags(tags)
        val eq = equalizer
        if (eq != null && (eq.sampleRate != inputAudioFormat.sampleRate || eq.channels != inputAudioFormat.channelCount)) {
            equalizer = null
            equalizerFor = null
        }
    }

    override fun onReset() {
        tags = ReplayGainTags()
        nextTags = ReplayGainTags()
        equalizer = null
        equalizerFor = null
    }

    override fun queueInput(inputBuffer: ByteBuffer) {
        val size = inputBuffer.remaining()
        if (size == 0) return
        val settings = sound()
        val eq = settings.eq.takeIf { it.active }
        val gainDb = replayGainDb(settings.replayGain, settings.replayGainPreampDb, tags)
        val output = replaceOutputBuffer(size)
        if (eq == null) {
            equalizer = null
            equalizerFor = null
            if (gainDb == null) {
                output.put(inputBuffer)
                output.flip()
                return
            }
        }
        val float = inputAudioFormat.encoding == C.ENCODING_PCM_FLOAT
        val count = size / if (float) 4 else 2
        if (samples.size < count) samples = DoubleArray(count)
        val input = inputBuffer.order(ByteOrder.nativeOrder())
        val gain = gainDb?.let { 10.0.pow(it / 20.0) } ?: 1.0
        if (float) {
            for (i in 0 until count) samples[i] = input.getFloat() * gain
        } else {
            for (i in 0 until count) samples[i] = input.getShort() / 32_768.0 * gain
        }
        if (eq != null) equalizer(eq).process(samples, count)
        if (float) {
            for (i in 0 until count) output.putFloat(samples[i].coerceIn(-1.0, 1.0).toFloat())
        } else {
            for (i in 0 until count) output.putShort((samples[i] * 32_768.0).roundToInt().coerceIn(-32_768, 32_767).toShort())
        }
        inputBuffer.position(inputBuffer.limit())
        output.flip()
    }

    private fun equalizer(settings: EqSettings): Equalizer {
        val eq = equalizer ?: Equalizer(inputAudioFormat.sampleRate, inputAudioFormat.channelCount).also { equalizer = it }
        if (settings != equalizerFor) {
            eq.set(settings)
            equalizerFor = settings
        }
        return eq
    }
}

/** Android's output with [processor] in front: each track's ReplayGain tags go to it with the track's format. */
@OptIn(UnstableApi::class)
class SoundSink(sink: AudioSink, private val processor: SoundProcessor) : ForwardingAudioSink(sink) {
    override fun configure(inputFormat: Format, specifiedBufferSize: Int, outputChannels: IntArray?) {
        processor.nextTags = replayGainTagsOf(inputFormat.metadata)
        super.configure(inputFormat, specifiedBufferSize, outputChannels)
    }
}

/** What the sound processing does to the phone's track now, for Now Playing: "EQ • ReplayGain −6.5 dB". */
fun soundEffects(sound: Sound, tags: ReplayGainTags): String? =
    listOfNotNull(
        "EQ".takeIf { sound.eq.active },
        replayGainDb(sound.replayGain, sound.replayGainPreampDb, tags)?.let { db ->
            String.format(Locale.ROOT, "ReplayGain %s%.1f dB", if (db < 0) "−" else "+", abs(db))
        },
    ).joinToString(" • ").ifEmpty { null }
