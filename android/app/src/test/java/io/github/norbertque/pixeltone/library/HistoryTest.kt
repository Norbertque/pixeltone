package io.github.norbertque.pixeltone.library

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class HistoryTest {
    private var now = 0L
    private val recorded = mutableListOf<Listen>()
    private val counter = PlayCounter({ now }, recorded::add)
    private var nextId = 1L

    private fun track(title: String, durationMs: Long) =
        Track(nextId++, title, "Artist", null, "Album", 7, durationMs, null, null, null, "Music/Album/", "$title.flac")

    private fun play(ms: Long) {
        counter.onPlaying(true)
        now += ms
        counter.onPlaying(false)
    }

    @Test
    fun `a listen counts after half the track or four minutes, never under 30 s`() {
        val short = track("Short", 50_000)
        counter.onTrack(short, playing = false)
        play(29_000) // half of it, but under 30 s
        counter.onTrack(null, playing = false)
        assertTrue(recorded.isEmpty())

        val song = track("Song", 200_000)
        counter.onTrack(song, playing = false)
        play(60_000)
        play(40_000) // 100 s: half
        counter.onTrack(null, playing = false)
        assertEquals(listOf("Song"), recorded.map { it.track.title })
        assertEquals(100_000L, recorded.single().heardMs)

        val long = track("Long", 30 * 60_000)
        counter.onTrack(long, playing = false)
        play(240_000) // 4 minutes of half an hour
        counter.flush()
        assertEquals(listOf("Song", "Long"), recorded.map { it.track.title })
    }

    @Test
    fun `the same track carries on across a rebuilt player, and starts over on repeat`() {
        val song = track("Song", 100_000)
        counter.onTrack(song, playing = true)
        now += 30_000
        // A new player (a DAC came up) reports the same track: still one listen.
        counter.onTrack(song, playing = true)
        now += 30_000
        counter.onTrack(song, playing = true, again = true)
        assertEquals(1, recorded.size)
        assertEquals(60_000L, recorded.single().heardMs)
    }

    @Test
    fun `history survives a restart`() {
        val file = File.createTempFile("history", ".jsonl").apply { delete() }
        val history = History(file)
        val listen = Listen(ListedTrack("Music/a.flac", "A", "Artist", "Album"), 1_000, 60_000)
        history.record(listen)
        history.record(listen.copy(startedAt = 2_000))
        val deadline = System.currentTimeMillis() + 5_000
        while (History(file).listens.value.size < 2 && System.currentTimeMillis() < deadline) Thread.sleep(20)
        assertEquals(history.listens.value, History(file).listens.value)
    }

    @Test
    fun `stats rank by plays and group names ignoring case and accents`() {
        fun listen(title: String, artist: String, at: Long, album: String = "Album") =
            Listen(ListedTrack("Music/$title.flac", title, artist, album), at, 60_000)
        val listens = listOf(
            listen("A", "Kryštof", 1_000),
            listen("A", "Kryštof", 2_000),
            listen("B", "kRYSTOF", 3_000),
            listen("C", "Other", 4_000, album = "Else"),
        )
        val all = listeningStats(listens, since = null)
        assertEquals(4, all.plays)
        assertEquals(240_000L, all.heardMs)
        assertEquals("A", all.tracks.first().item.title)
        assertEquals(2, all.tracks.first().plays)
        assertEquals(3, all.artists.first().plays)
        assertEquals("Kryštof", all.artists.first().item)
        assertEquals(listOf("C", "B", "A", "A"), all.recent.map { it.track.title })
        assertEquals(2, listeningStats(listens, since = 3_000).plays)
    }
}
