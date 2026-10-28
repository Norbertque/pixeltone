package io.github.norbertque.pixeltone.ui

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.ui.library.AlbumScreen
import io.github.norbertque.pixeltone.ui.library.ArtistScreen
import io.github.norbertque.pixeltone.ui.library.LibraryScreen

private val AUDIO_PERMISSION =
    if (Build.VERSION.SDK_INT >= 33) Manifest.permission.READ_MEDIA_AUDIO else Manifest.permission.READ_EXTERNAL_STORAGE

@Composable
fun PixeltoneRoot() {
    val context = LocalContext.current
    var allowed by remember { mutableStateOf(context.mayReadAudio()) }
    // Access can be given in the system settings while the app waits in the background.
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { allowed = context.mayReadAudio() }
    if (allowed) {
        LaunchedEffect(Unit) { (context.applicationContext as PixeltoneApp).library.start() }
        Navigation()
    } else {
        AccessScreen(onAllowed = { allowed = true })
    }
}

@Composable
private fun Navigation() {
    val nav = rememberNavController()
    NavHost(nav, startDestination = "library") {
        composable("library") {
            LibraryScreen(
                onAlbum = { nav.navigate("album/${Uri.encode(it)}") },
                onArtist = { nav.navigate("artist/${Uri.encode(it)}") },
            )
        }
        composable("album/{key}") { entry ->
            AlbumScreen(
                key = entry.arguments?.getString("key").orEmpty(),
                onBack = { nav.popBackStack() },
                onArtist = { nav.navigate("artist/${Uri.encode(it)}") },
            )
        }
        composable("artist/{key}") { entry ->
            ArtistScreen(
                key = entry.arguments?.getString("key").orEmpty(),
                onBack = { nav.popBackStack() },
                onAlbum = { nav.navigate("album/${Uri.encode(it)}") },
            )
        }
    }
}

@Composable
internal fun AccessScreen(onAllowed: () -> Unit) {
    val context = LocalContext.current
    var refused by rememberSaveable { mutableStateOf(false) }
    val request = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        if (granted) onAllowed() else refused = true
    }
    // Once Android stops asking ("don't ask again"), only the app's settings can change it.
    val askable = !refused ||
        context.findActivity()?.let { ActivityCompat.shouldShowRequestPermissionRationale(it, AUDIO_PERMISSION) } == true
    Surface(Modifier.fillMaxSize()) {
        Column(
            Modifier.fillMaxSize().safeDrawingPadding().padding(32.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            PixeltoneGlyph(Modifier.size(56.dp))
            Spacer(Modifier.height(24.dp))
            Text(stringResource(R.string.access_title), style = MaterialTheme.typography.headlineSmall)
            Spacer(Modifier.height(12.dp))
            Text(
                stringResource(if (askable) R.string.access_text else R.string.access_denied),
                textAlign = TextAlign.Center,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.height(24.dp))
            if (askable) {
                Button(onClick = { request.launch(AUDIO_PERMISSION) }) { Text(stringResource(R.string.access_allow)) }
            } else {
                Button(onClick = { context.openAppSettings() }) { Text(stringResource(R.string.access_settings)) }
            }
        }
    }
}

private fun Context.mayReadAudio() =
    ContextCompat.checkSelfPermission(this, AUDIO_PERMISSION) == PackageManager.PERMISSION_GRANTED

private fun Context.openAppSettings() {
    startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", packageName, null)))
}

private tailrec fun Context.findActivity(): Activity? =
    when (this) {
        is Activity -> this
        is ContextWrapper -> baseContext.findActivity()
        else -> null
    }
