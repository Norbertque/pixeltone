package io.github.norbertque.pixeltone.ui.settings

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ColourTest {
    @Test
    fun `a typed hex colour is read with or without its hash, and written back`() {
        assertEquals(0xFF7E57C2.toInt(), parseHexColour("#7E57C2"))
        assertEquals(0xFF7E57C2.toInt(), parseHexColour(" 7e57c2 "))
        assertNull(parseHexColour("#7E57C"))
        assertNull(parseHexColour("lilac"))
        assertEquals("#7E57C2", hexOf(0xFF7E57C2.toInt()))
    }
}
