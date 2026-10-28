package io.github.norbertque.pixeltone.update

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

// Robolectric for org.json, which the plain JVM only has as stubs.
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35])
class UpdateFeedTest {
    private val hash = "a".repeat(64)

    @Test
    fun `a complete feed is read`() {
        val info = parseUpdateInfo(
            """{"versionCode": 12, "versionName": "0.1.12", "url": "https://example.com/a.apk", "sha256": "${hash.uppercase()}"}""",
        )
        assertEquals(UpdateInfo(12, "0.1.12", "https://example.com/a.apk", hash), info)
    }

    @Test
    fun `an incomplete or unsafe feed is ignored`() {
        assertNull(parseUpdateInfo("""{"versionCode": 12, "versionName": "0.1.12", "url": "https://example.com/a.apk"}"""))
        assertNull(parseUpdateInfo("""{"versionCode": 12, "versionName": "0.1.12", "url": "http://example.com/a.apk", "sha256": "$hash"}"""))
        assertNull(parseUpdateInfo("""{"versionCode": 12, "versionName": "0.1.12", "url": "https://example.com/a.apk", "sha256": "xyz"}"""))
        assertNull(parseUpdateInfo("<html>"))
    }

    @Test
    fun `sha256 of a file`() {
        val file = File.createTempFile("pixeltone", ".bin").apply { writeText("abc") }
        assertEquals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", sha256(file))
        file.delete()
    }
}
