package io.github.norbertque.pixeltone

import android.app.SearchManager
import android.content.Intent
import android.os.Bundle
import android.provider.MediaStore
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import io.github.norbertque.pixeltone.remote.parseJamLink
import io.github.norbertque.pixeltone.ui.PixeltoneRoot

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        // Not again when the activity is recreated (rotation): the intent was handled.
        if (savedInstanceState == null) {
            openLink(intent)
            playAsked(intent)
        }
        setContent {
            // The theme takes its colours from what plays: PixeltoneRoot sets it up.
            PixeltoneRoot()
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        openLink(intent)
        playAsked(intent)
    }

    /** "Play … on Pixeltone", said to the assistant. */
    private fun playAsked(intent: Intent?) {
        if (intent?.action != MediaStore.INTENT_ACTION_MEDIA_PLAY_FROM_SEARCH) return
        (application as PixeltoneApp).player.playAsked(intent.getStringExtra(SearchManager.QUERY).orEmpty(), intent.extras)
    }

    /** A jam's QR code or link (the page, or the page's "Open in Pixeltone" button). */
    private fun openLink(intent: Intent?) {
        if (intent?.action != Intent.ACTION_VIEW) return
        val link = intent.dataString ?: return
        val invite = parseJamLink(link)
        if (invite == null) {
            Toast.makeText(this, R.string.jam_link_invalid, Toast.LENGTH_LONG).show()
            return
        }
        (application as PixeltoneApp).jam.open(invite)
    }
}
