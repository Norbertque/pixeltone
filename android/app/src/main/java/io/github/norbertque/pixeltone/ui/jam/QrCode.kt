package io.github.norbertque.pixeltone.ui.jam

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.google.zxing.qrcode.QRCodeWriter
import com.google.zxing.qrcode.decoder.ErrorCorrectionLevel

/** [text] as a QR code: always black on white, whatever the theme, so cameras read it. */
@Composable
fun QrCode(text: String, description: String, modifier: Modifier = Modifier) {
    val matrix = remember(text) {
        QRCodeWriter().encode(
            text,
            BarcodeFormat.QR_CODE,
            0,
            0,
            mapOf(EncodeHintType.MARGIN to 0, EncodeHintType.ERROR_CORRECTION to ErrorCorrectionLevel.M),
        )
    }
    Canvas(
        modifier
            .aspectRatio(1f)
            .clip(RoundedCornerShape(16.dp))
            .background(Color.White)
            .padding(16.dp)
            .semantics { contentDescription = description },
    ) {
        val module = size.width / matrix.width
        for (y in 0 until matrix.height) {
            var x = 0
            while (x < matrix.width) {
                if (!matrix[x, y]) {
                    x++
                    continue
                }
                val start = x
                while (x < matrix.width && matrix[x, y]) x++
                // A hair wider, so neighbouring runs don't leave seams.
                drawRect(Color.Black, Offset(start * module, y * module), Size((x - start) * module + 0.5f, module + 0.5f))
            }
        }
    }
}
