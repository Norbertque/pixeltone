package io.github.norbertque.pixeltone.ui.theme

import androidx.compose.ui.graphics.Color
import org.junit.Assert.assertEquals
import org.junit.Test

/** The scheme comes out as the Windows UI builds it (material-color-utilities, tonal spot, 2025 spec). */
class SchemeTest {
    @Test
    fun `lilac gives the colours Windows shows`() {
        assertEquals(Color(0xFF685688), colorSchemeFor(DEFAULT_SEED, dark = false).primary)
        assertEquals(Color(0xFFD0BFEB), colorSchemeFor(DEFAULT_SEED, dark = true).primary)
        assertEquals(Color(0xFFF3EBF5), colorSchemeFor(DEFAULT_SEED, dark = false).surfaceContainer)
    }
}
