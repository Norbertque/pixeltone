package io.github.norbertque.pixeltone.remote

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Messages as Pixeltone (pp-remote) sends them, and commands as it reads them. */
class RemoteProtocolTest {
    @Test
    fun `reads the messages a PC sends`() {
        val hello = PcMessage.parse(
            """{"type":"hello","server":{"app":"Pixeltone","id":"abc","name":"DESKTOP-1","version":"0.3.120","protocol":1,"fingerprint":"0a1b"}}""",
        )
        assertEquals("DESKTOP-1", (hello as PcMessage.Hello).server.name)

        val state = PcMessage.parse(
            """{"type":"state","state":{"status":"playing","track":{"path":"http://192.168.1.20:8080/pc/file/42?auth=t",""" +
                """"title":"Song","artist":"Artist","album":null,"durationMs":201533.33,"codec":"flac",""" +
                """"source":{"sampleRate":96000,"bits":24,"channels":2},"lossless":true,"dsd":false,"coverKey":"remote/12"},""" +
                """"currentId":7,"queueIndex":2,"queueLength":10,"queueVersion":3,"positionMs":61000.5,"repeat":"all",""" +
                """"shuffle":false,"output":{"deviceName":"DAC","requested":"exclusive","active":"exclusive",""" +
                """"fallbackReason":null,"badge":{"label":"Exclusive • 24-bit/96 kHz • Bit-perfect","bitPerfect":true},"stats":{}}}}""",
        ) as PcMessage.State
        assertTrue(state.state.isPlaying)
        assertTrue(state.state.track!!.path.endsWith("/pc/file/42?auth=t"))
        assertEquals(201533.33, state.state.track.durationMs!!, 0.01)
        assertEquals(2, state.state.queueIndex)
        assertEquals(61000.5, state.state.positionMs, 0.01)
        assertTrue(state.state.output!!.badge!!.label.contains("Bit-perfect"))

        val stopped = PcMessage.parse(
            """{"type":"state","state":{"status":"stopped","track":null,"currentId":null,"queueIndex":null,""" +
                """"queueLength":0,"queueVersion":0,"positionMs":0.0,"repeat":"off","shuffle":false,"output":null}}""",
        ) as PcMessage.State
        assertNull(stopped.state.track)

        val position = PcMessage.parse("""{"type":"position","positionMs":1234.5}""")
        assertEquals(1234.5, (position as PcMessage.Position).positionMs, 0.01)

        val volume = PcMessage.parse(
            """{"type":"volume","volume":{"kind":"device","db":-20.5,"min":-127.0,"max":0.0,"step":0.5,"muted":false}}""",
        ) as PcMessage.Volume
        assertTrue(volume.volume!!.adjustable)
        assertNull((PcMessage.parse("""{"type":"volume","volume":null}""") as PcMessage.Volume).volume)
        val fixed = PcMessage.parse(
            """{"type":"volume","volume":{"kind":"fixed","db":0.0,"min":0.0,"max":0.0,"step":1.0,"muted":false}}""",
        ) as PcMessage.Volume
        assertFalse(fixed.volume!!.adjustable)

        val error = PcMessage.parse("""{"type":"error","code":"refused","message":"Nothing to play."}""")
        assertEquals("Nothing to play.", (error as PcMessage.Error).message)
        assertEquals(PcMessage.LibraryChanged, PcMessage.parse("""{"type":"libraryChanged"}"""))
    }

    @Test
    fun `ignores what it doesn't know`() {
        assertNull(PcMessage.parse("""{"type":"somethingNew","x":1}"""))
        assertNull(PcMessage.parse("not json"))
    }

    @Test
    fun `writes commands as documented`() {
        fun json(text: String): JsonObject = Json.parseToJsonElement(text).jsonObject

        assertEquals("toggle", json(PcCommand.simple("toggle"))["cmd"]?.jsonPrimitive?.content)
        assertEquals("61000", json(PcCommand.seek(61000))["ms"]?.jsonPrimitive?.content)
        assertEquals("one", json(PcCommand.setRepeat("one"))["mode"]?.jsonPrimitive?.content)
        assertEquals("-12.5", json(PcCommand.setVolume(-12.5f))["db"]?.jsonPrimitive?.content)

        val load = json(
            PcCommand.load(
                items = listOf(
                    HandOverItem(url = "http://10.0.0.2:8080/pc/file/1?auth=t", title = "One", durationMs = 1000),
                    HandOverItem(url = "http://10.0.0.2:8080/pc/file/2?auth=t"),
                ),
                start = 1,
                positionMs = 500,
                play = false,
                repeat = "all",
            ),
        )
        assertEquals("load", load["cmd"]?.jsonPrimitive?.content)
        assertEquals("1", load["start"]?.jsonPrimitive?.content)
        assertEquals("false", load["play"]?.jsonPrimitive?.content)
        val items = load["items"]!!.jsonArray
        assertEquals(2, items.size)
        assertEquals("1000", items[0].jsonObject["durationMs"]?.jsonPrimitive?.content)
        // Unknown fields stay out rather than going over as null.
        assertEquals(setOf("url"), items[1].jsonObject.keys)
    }

    @Test
    fun `reads addresses as Pixeltone shows them`() {
        assertEquals("192.168.1.20" to 47821, parseAddress("192.168.1.20:47821"))
        assertEquals("192.168.1.20" to RemoteProtocol.DEFAULT_PORT, parseAddress(" 192.168.1.20 "))
        assertEquals("desktop.local" to 5000, parseAddress("https://desktop.local:5000/"))
        assertEquals("fe80::1" to 47821, parseAddress("[fe80::1]:47821"))
        assertEquals("fd00::7" to RemoteProtocol.DEFAULT_PORT, parseAddress("fd00::7"))
        assertNull(parseAddress(""))
        assertNull(parseAddress("192.168.1.20:port"))
        assertNull(parseAddress("192.168.1.20:70000"))
        assertNull(parseAddress("two words"))
    }
}
