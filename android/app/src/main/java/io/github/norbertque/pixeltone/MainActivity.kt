package io.github.norbertque.pixeltone

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import io.github.norbertque.pixeltone.ui.PixeltoneRoot
import io.github.norbertque.pixeltone.ui.theme.PixeltoneTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent {
            PixeltoneTheme {
                PixeltoneRoot()
            }
        }
    }
}
