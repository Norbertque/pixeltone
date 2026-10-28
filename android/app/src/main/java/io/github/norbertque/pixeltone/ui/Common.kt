package io.github.norbertque.pixeltone.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import io.github.norbertque.pixeltone.R
import java.util.Locale

/** The three bars of Pixeltone's icon. */
@Composable
fun PixeltoneGlyph(modifier: Modifier = Modifier, color: Color = MaterialTheme.colorScheme.primary) {
    Canvas(modifier) {
        val scale = minOf(size.width / GLYPH_WIDTH, size.height / GLYPH_HEIGHT)
        val left = (size.width - GLYPH_WIDTH * scale) / 2
        val top = (size.height - GLYPH_HEIGHT * scale) / 2
        val width = BAR_WIDTH * scale
        for ((x, y, height) in BARS) {
            drawRoundRect(color, Offset(left + x * scale, top + y * scale), Size(width, height * scale), CornerRadius(width / 2))
        }
    }
}

// The icon's 1024 grid (src-tauri/icons/app-icon.svg), from the glyph's top left corner.
private const val GLYPH_WIDTH = 424f
private const val GLYPH_HEIGHT = 460f
private const val BAR_WIDTH = 104f
private val BARS = listOf(Triple(0f, 130f, 200f), Triple(160f, 0f, 460f), Triple(320f, 90f, 280f))

@Composable
fun BackButton(onBack: () -> Unit) {
    IconButton(onClick = onBack) {
        Icon(Icons.AutoMirrored.Filled.ArrowBack, stringResource(R.string.back))
    }
}

@Composable
fun Loading(modifier: Modifier = Modifier) {
    Box(modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        CircularProgressIndicator()
    }
}

@Composable
fun Message(text: String, modifier: Modifier = Modifier) {
    Box(modifier.fillMaxSize().padding(32.dp), contentAlignment = Alignment.Center) {
        Text(text, textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

/** 3:07, or 1:02:03 from an hour on. */
fun formatDuration(ms: Long): String {
    val seconds = (ms + 500) / 1000
    val h = seconds / 3600
    val m = seconds % 3600 / 60
    val s = seconds % 60
    return if (h > 0) {
        String.format(Locale.ROOT, "%d:%02d:%02d", h, m, s)
    } else {
        String.format(Locale.ROOT, "%d:%02d", m, s)
    }
}
