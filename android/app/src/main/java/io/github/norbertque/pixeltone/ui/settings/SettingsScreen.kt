package io.github.norbertque.pixeltone.ui.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.VolumeUp
import androidx.compose.material.icons.filled.ErrorOutline
import androidx.compose.material.icons.filled.GraphicEq
import androidx.compose.material.icons.filled.Usb
import androidx.compose.material.icons.filled.WarningAmber
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.ui.BackButton
import io.github.norbertque.pixeltone.usb.UsbExclusiveState
import io.github.norbertque.pixeltone.usb.UsbRememberedDevice
import io.github.norbertque.pixeltone.usb.bestFormat
import io.github.norbertque.pixeltone.usb.formatDb
import io.github.norbertque.pixeltone.usb.formatRate
import io.github.norbertque.pixeltone.usb.usbBadge
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** USB output as Settings shows it. */
data class UsbPanel(
    val enabled: Boolean,
    val state: UsbExclusiveState,
    /** The software volume, while it's in play (a DAC without a volume of its own). */
    val softwareVolumeDb: Float?,
    /** The DAC's own volume, 0..1, when it has one. */
    val hardwareVolume: Float?,
    val fixedVolume: Boolean,
    val loudnessAcknowledged: Boolean,
)

@Composable
fun SettingsScreen(app: PixeltoneApp, onBack: () -> Unit) {
    val usb = app.usb
    val settings = app.usbSettings
    val state by usb.state.collectAsStateWithLifecycle()
    val enabled by settings.exclusiveEnabled.collectAsStateWithLifecycle()
    val fixed by settings.fixedVolumeOutput.collectAsStateWithLifecycle()
    val acknowledged by settings.maxVolumeAcknowledged.collectAsStateWithLifecycle()
    val softwareDb by usb.softwareVolumeDb.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    var hardware by remember { mutableStateOf<Float?>(null) }
    val session = usb.activeSession
    // The DAC's volume is asked over USB: off the main thread, again for each new session.
    LaunchedEffect(session) { hardware = withContext(Dispatchers.IO) { usb.hardwareVolumeFraction() } }
    SettingsContent(
        usb = UsbPanel(enabled, state, softwareDb, hardware, fixed, acknowledged),
        onBack = onBack,
        onEnabled = settings::setExclusiveEnabled,
        onRetry = {
            when (val s = state) {
                is UsbExclusiveState.PermissionDenied -> usb.retryPermission(s.device)
                else -> usb.refresh()
            }
        },
        onHardwareVolume = { fraction ->
            hardware = fraction
            scope.launch(Dispatchers.IO) { usb.setHardwareVolume(fraction) }
        },
        onSoftwareVolume = { usb.setSoftwareVolumeDb(it) },
        onAcknowledgeLoudness = { settings.setMaxVolumeAcknowledged(true) },
        onFixedVolume = settings::setFixedVolumeOutput,
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsContent(
    usb: UsbPanel,
    onBack: () -> Unit,
    onEnabled: (Boolean) -> Unit,
    onRetry: () -> Unit,
    onHardwareVolume: (Float) -> Unit,
    onSoftwareVolume: (Float) -> Unit,
    onAcknowledgeLoudness: () -> Unit,
    onFixedVolume: (Boolean) -> Unit,
) {
    Scaffold(
        topBar = { TopAppBar(title = { Text(stringResource(R.string.settings)) }, navigationIcon = { BackButton(onBack) }) },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState())) {
            SectionTitle(stringResource(R.string.usb_title))
            ListItem(
                headlineContent = { Text(stringResource(R.string.usb_exclusive)) },
                supportingContent = { Text(stringResource(R.string.usb_exclusive_text)) },
                trailingContent = { Switch(checked = usb.enabled, onCheckedChange = onEnabled) },
            )
            if (usb.enabled) {
                Status(usb.state, onRetry)
                val ready = usb.state is UsbExclusiveState.Ready || usb.state is UsbExclusiveState.Active
                if (ready) Volume(usb, onHardwareVolume, onSoftwareVolume, onAcknowledgeLoudness, onFixedVolume)
            }
            HorizontalDivider(Modifier.padding(top = 16.dp))
            Text(
                stringResource(R.string.usb_licence),
                Modifier.padding(16.dp),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun SectionTitle(text: String) {
    Text(
        text,
        Modifier.padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 4.dp),
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
    )
}

@Composable
private fun Status(state: UsbExclusiveState, onRetry: () -> Unit) {
    val (icon, text, action) = when (state) {
        UsbExclusiveState.Disabled, UsbExclusiveState.NoDevice ->
            Triple(Icons.Default.Usb, stringResource(R.string.usb_no_device), null)
        is UsbExclusiveState.DeviceDetected ->
            Triple(Icons.Default.Usb, stringResource(R.string.usb_permission, state.device.displayName), null)
        is UsbExclusiveState.PermissionPending ->
            Triple(Icons.Default.Usb, stringResource(R.string.usb_permission, state.device.displayName), null)
        is UsbExclusiveState.PermissionDenied ->
            Triple(Icons.Default.ErrorOutline, stringResource(R.string.usb_denied, state.device.displayName), R.string.usb_ask_again)
        is UsbExclusiveState.Ready ->
            Triple(Icons.Default.Usb, stringResource(R.string.usb_ready, state.device.displayName, bestFormat(state.capabilities)), null)
        is UsbExclusiveState.Active ->
            Triple(Icons.Default.GraphicEq, stringResource(R.string.usb_active, state.device.displayName, usbBadge(state).orEmpty()), null)
        is UsbExclusiveState.Error ->
            Triple(Icons.Default.ErrorOutline, state.message, if (state.recoverable) R.string.usb_try_again else null)
    }
    val error = state is UsbExclusiveState.Error || state is UsbExclusiveState.PermissionDenied
    Card(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (error) MaterialTheme.colorScheme.errorContainer else MaterialTheme.colorScheme.secondaryContainer,
        ),
    ) {
        Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            Icon(icon, null)
            Spacer(Modifier.size(16.dp))
            Column(Modifier.weight(1f)) {
                Text(text)
                val capabilities = (state as? UsbExclusiveState.Ready)?.capabilities ?: (state as? UsbExclusiveState.Active)?.capabilities
                if (capabilities != null) {
                    Text(
                        stringResource(R.string.usb_rates, capabilities.allSampleRatesHz.joinToString(", ") { formatRate(it) }),
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }
            if (action != null) TextButton(onClick = onRetry) { Text(stringResource(action)) }
        }
    }
}

@Composable
private fun Volume(
    usb: UsbPanel,
    onHardwareVolume: (Float) -> Unit,
    onSoftwareVolume: (Float) -> Unit,
    onAcknowledgeLoudness: () -> Unit,
    onFixedVolume: (Boolean) -> Unit,
) {
    val hasHardwareVolume = when (val s = usb.state) {
        is UsbExclusiveState.Ready -> s.capabilities.volume != null
        is UsbExclusiveState.Active -> s.hardwareVolume
        else -> false
    }
    SectionTitle(stringResource(R.string.usb_volume))
    when {
        hasHardwareVolume -> VolumeSlider(
            label = stringResource(R.string.usb_dac_volume),
            value = usb.hardwareVolume ?: 0.5f,
            range = 0f..1f,
            shown = "${((usb.hardwareVolume ?: 0.5f) * 100).toInt()} %",
            onChange = onHardwareVolume,
        )
        !usb.fixedVolume && usb.softwareVolumeDb != null -> {
            val cap = if (usb.loudnessAcknowledged) UsbRememberedDevice.MAX_SOFTWARE_VOLUME_DB else UsbRememberedDevice.UNACKNOWLEDGED_CAP_DB
            VolumeSlider(
                label = stringResource(R.string.usb_software_volume),
                value = usb.softwareVolumeDb.coerceAtMost(cap),
                range = UsbRememberedDevice.MIN_SOFTWARE_VOLUME_DB..cap,
                shown = formatDb(usb.softwareVolumeDb),
                onChange = onSoftwareVolume,
            )
            Text(
                stringResource(R.string.usb_software_volume_text),
                Modifier.padding(horizontal = 16.dp),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            if (!usb.loudnessAcknowledged) Loudness(onAcknowledgeLoudness)
        }
    }
    if (!hasHardwareVolume) {
        ListItem(
            headlineContent = { Text(stringResource(R.string.usb_fixed)) },
            supportingContent = { Text(stringResource(R.string.usb_fixed_text)) },
            trailingContent = { Switch(checked = usb.fixedVolume, onCheckedChange = onFixedVolume) },
        )
    }
}

@Composable
private fun VolumeSlider(label: String, value: Float, range: ClosedFloatingPointRange<Float>, shown: String, onChange: (Float) -> Unit) {
    Column(Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Icon(Icons.AutoMirrored.Filled.VolumeUp, null, Modifier.size(20.dp))
            Spacer(Modifier.size(12.dp))
            Text(label, Modifier.weight(1f))
            Text(shown, style = MaterialTheme.typography.labelLarge)
        }
        Slider(value = value, onValueChange = onChange, valueRange = range)
    }
}

@Composable
private fun Loudness(onAcknowledge: () -> Unit) {
    Card(
        Modifier.fillMaxWidth().padding(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.tertiaryContainer),
    ) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Default.WarningAmber, null)
                Spacer(Modifier.size(12.dp))
                Text(stringResource(R.string.usb_loud_title), style = MaterialTheme.typography.titleSmall)
            }
            Text(stringResource(R.string.usb_loud_text), style = MaterialTheme.typography.bodyMedium)
            Spacer(Modifier.height(4.dp))
            FilledTonalButton(onClick = onAcknowledge) { Text(stringResource(R.string.usb_loud_allow)) }
        }
    }
}
