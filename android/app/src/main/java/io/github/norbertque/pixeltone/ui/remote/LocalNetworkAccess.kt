package io.github.norbertque.pixeltone.ui.remote

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.core.app.ActivityCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import io.github.norbertque.pixeltone.remote.LOCAL_NETWORK_PERMISSION
import io.github.norbertque.pixeltone.remote.mayUseLocalNetwork
import io.github.norbertque.pixeltone.ui.findActivity
import io.github.norbertque.pixeltone.ui.openAppSettings

/** Whether Pixeltone may use the Wi-Fi's devices ([mayUseLocalNetwork]), and a way to ask. */
class LocalNetworkAccess(val allowed: Boolean, val ask: () -> Unit)

/**
 * Asks Android for the Wi-Fi's devices, as finding and playing on PCs and jams need on Android
 * 17; once refused for good, [LocalNetworkAccess.ask] opens Android's page about Pixeltone
 * instead. [onAnswer] hears whether it was given.
 */
@Composable
fun rememberLocalNetworkAccess(onAnswer: (Boolean) -> Unit = {}): LocalNetworkAccess {
    val context = LocalContext.current
    var allowed by remember { mutableStateOf(mayUseLocalNetwork(context)) }
    var asked by rememberSaveable { mutableStateOf(false) }
    val answer by rememberUpdatedState(onAnswer)
    // It can be given in Android's settings while Pixeltone waits in the background.
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { allowed = mayUseLocalNetwork(context) }
    val request = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        allowed = granted
        asked = true
        answer(granted)
    }
    return LocalNetworkAccess(allowed) {
        val activity = context.findActivity()
        val refusedForGood = asked && activity != null && !ActivityCompat.shouldShowRequestPermissionRationale(activity, LOCAL_NETWORK_PERMISSION)
        if (refusedForGood) context.openAppSettings() else request.launch(LOCAL_NETWORK_PERMISSION)
    }
}
