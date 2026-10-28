package io.github.norbertque.pixeltone.ui.remote

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Computer
import androidx.compose.material.icons.filled.LinkOff
import androidx.compose.material.icons.filled.Smartphone
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.remote.FoundPc
import io.github.norbertque.pixeltone.remote.PairedPc
import io.github.norbertque.pixeltone.remote.PcInfo
import io.github.norbertque.pixeltone.remote.RemoteControl
import io.github.norbertque.pixeltone.remote.parseAddress
import kotlinx.coroutines.launch

/** A PC to play on: paired, found on the Wi-Fi, or both. */
data class PcChoice(val key: String, val name: String, val found: FoundPc?, val paired: PairedPc?)

/** Paired PCs first (with where they were found, if they were), then the others found. */
fun pcChoices(found: List<FoundPc>, paired: List<PairedPc>): List<PcChoice> {
    val pairedChoices = paired.map { pc ->
        PcChoice(pc.id, pc.name, found.firstOrNull { it.id == pc.id } ?: found.firstOrNull { it.id == null && it.host == pc.host }, pc)
    }
    val others = found.filter { f -> pairedChoices.none { it.found == f } }.map { PcChoice(it.serviceName, it.name, it, null) }
    return pairedChoices + others
}

/** A PC to pair with, at an address. */
data class PairTarget(val host: String, val port: Int, val name: String)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PlayOnSheet(remote: RemoteControl, onDismiss: () -> Unit) {
    val found by remote.discovery.found.collectAsStateWithLifecycle()
    val paired by remote.paired.pcs.collectAsStateWithLifecycle()
    val session by remote.connection.session.collectAsStateWithLifecycle()
    DisposableEffect(Unit) {
        remote.discovery.start()
        onDispose { remote.discovery.stop() }
    }
    var pairing by remember { mutableStateOf<PairTarget?>(null) }
    var byAddress by remember { mutableStateOf(false) }
    var forgetting by remember { mutableStateOf<PairedPc?>(null) }
    ModalBottomSheet(onDismissRequest = onDismiss) {
        PlayOnContent(
            choices = pcChoices(found, paired),
            activeId = session?.pc?.id,
            connected = session?.connected == true,
            onPhone = {
                remote.playHere()
                onDismiss()
            },
            onPc = { choice ->
                val pc = choice.paired
                if (pc != null) {
                    remote.playOn(pc, choice.found)
                    onDismiss()
                } else {
                    choice.found?.let { pairing = PairTarget(it.host, it.port, it.name) }
                }
            },
            onForget = { forgetting = it },
            onAddress = { byAddress = true },
        )
    }
    // The dialogs stay outside the sheet, so the keyboard can't take them away with it.
    pairing?.let { target ->
        PairDialog(
            target = target,
            onPair = { code -> remote.pair(target.host, target.port, code) },
            onPaired = { pc ->
                pairing = null
                remote.playOn(pc)
                onDismiss()
            },
            onDismiss = { pairing = null },
        )
    }
    if (byAddress) {
        AddressDialog(
            onLookUp = remote::lookUp,
            onFound = { host, port, info ->
                byAddress = false
                val known = remote.paired.find(info.id)
                if (known != null) {
                    remote.playOn(known, FoundPc(info.name, info.id, info.name, host, port))
                    onDismiss()
                } else {
                    pairing = PairTarget(host, port, info.name)
                }
            },
            onDismiss = { byAddress = false },
        )
    }
    forgetting?.let { pc ->
        AlertDialog(
            onDismissRequest = { forgetting = null },
            title = { Text(stringResource(R.string.forget_title, pc.name)) },
            text = { Text(stringResource(R.string.forget_text)) },
            confirmButton = {
                TextButton(onClick = {
                    remote.forget(pc)
                    forgetting = null
                }) { Text(stringResource(R.string.forget)) }
            },
            dismissButton = { TextButton(onClick = { forgetting = null }) { Text(stringResource(R.string.cancel)) } },
        )
    }
}

/** Where to play: this phone or a PC; [activeId] is the PC playing now, if any. */
@Composable
fun PlayOnContent(
    choices: List<PcChoice>,
    activeId: String?,
    connected: Boolean,
    onPhone: () -> Unit,
    onPc: (PcChoice) -> Unit,
    onForget: (PairedPc) -> Unit,
    onAddress: () -> Unit,
) {
    Column(Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
        Text(
            stringResource(R.string.play_on),
            style = MaterialTheme.typography.titleLarge,
            modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
        )
        Device(Icons.Default.Smartphone, stringResource(R.string.this_phone), null, selected = activeId == null, onClick = onPhone)
        for (choice in choices) {
            val selected = activeId != null && choice.paired?.id == activeId
            val detail = stringResource(
                when {
                    selected && !connected -> R.string.pc_connecting
                    selected -> R.string.pc_playing
                    choice.paired != null && choice.found != null -> R.string.pc_here
                    choice.paired != null -> R.string.pc_away
                    else -> R.string.pc_new
                },
            )
            Device(
                Icons.Default.Computer,
                choice.name,
                detail,
                selected = selected,
                onClick = { onPc(choice) },
                onForget = choice.paired?.takeIf { !selected }?.let { pc -> { onForget(pc) } },
            )
        }
        TextButton(onClick = onAddress, modifier = Modifier.padding(horizontal = 12.dp, vertical = 4.dp)) {
            Icon(Icons.Default.Add, null, Modifier.size(18.dp))
            Spacer(Modifier.width(8.dp))
            Text(stringResource(R.string.add_pc_by_address))
        }
    }
}

@Composable
private fun Device(
    icon: ImageVector,
    name: String,
    detail: String?,
    selected: Boolean,
    onClick: () -> Unit,
    onForget: (() -> Unit)? = null,
) {
    val tint = if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
    ListItem(
        headlineContent = { Text(name, color = if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface) },
        supportingContent = detail?.let { { Text(it) } },
        leadingContent = { Icon(icon, null, tint = tint) },
        trailingContent = when {
            selected -> {
                { Icon(Icons.Default.Check, null, tint = MaterialTheme.colorScheme.primary) }
            }
            onForget != null -> {
                {
                    IconButton(onClick = onForget) {
                        Icon(Icons.Default.LinkOff, stringResource(R.string.forget_title, name))
                    }
                }
            }
            else -> null
        },
        colors = ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
        modifier = Modifier.clickable(onClick = onClick),
    )
}

/** Asks for the six-digit code the PC shows and pairs with it. */
@Composable
fun PairDialog(target: PairTarget, onPair: suspend (String) -> PairedPc, onPaired: (PairedPc) -> Unit, onDismiss: () -> Unit) {
    var code by rememberSaveable { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val failed = stringResource(R.string.pair_failed)
    val pair = {
        busy = true
        scope.launch {
            runCatching { onPair(code) }
                .onSuccess(onPaired)
                .onFailure {
                    error = it.message ?: failed
                    busy = false
                }
        }
        Unit
    }
    AlertDialog(
        onDismissRequest = { if (!busy) onDismiss() },
        title = { Text(stringResource(R.string.pair_title, target.name)) },
        text = {
            Column {
                Text(stringResource(R.string.pair_text))
                Spacer(Modifier.height(16.dp))
                OutlinedTextField(
                    value = code,
                    onValueChange = {
                        code = it.filter(Char::isDigit).take(6)
                        error = null
                    },
                    label = { Text(stringResource(R.string.pair_code)) },
                    singleLine = true,
                    enabled = !busy,
                    isError = error != null,
                    supportingText = error?.let { { Text(it) } },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Done),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        },
        confirmButton = {
            TextButton(onClick = pair, enabled = code.length == 6 && !busy) {
                if (busy) CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp) else Text(stringResource(R.string.pair))
            }
        },
        dismissButton = { TextButton(onClick = onDismiss, enabled = !busy) { Text(stringResource(R.string.cancel)) } },
    )
}

/** Finds a PC by the address Pixeltone shows in its settings. */
@Composable
fun AddressDialog(onLookUp: suspend (String, Int) -> PcInfo, onFound: (String, Int, PcInfo) -> Unit, onDismiss: () -> Unit) {
    var text by rememberSaveable { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val invalid = stringResource(R.string.address_invalid)
    val failed = stringResource(R.string.pair_failed)
    val lookUp = {
        val address = parseAddress(text)
        if (address == null) {
            error = invalid
        } else {
            busy = true
            scope.launch {
                runCatching { onLookUp(address.first, address.second) }
                    .onSuccess { onFound(address.first, address.second, it) }
                    .onFailure {
                        error = it.message ?: failed
                        busy = false
                    }
            }
        }
        Unit
    }
    AlertDialog(
        onDismissRequest = { if (!busy) onDismiss() },
        title = { Text(stringResource(R.string.add_pc_by_address)) },
        text = {
            Column {
                Text(stringResource(R.string.address_text))
                Spacer(Modifier.height(16.dp))
                OutlinedTextField(
                    value = text,
                    onValueChange = {
                        text = it
                        error = null
                    },
                    label = { Text(stringResource(R.string.address_hint)) },
                    singleLine = true,
                    enabled = !busy,
                    isError = error != null,
                    supportingText = error?.let { { Text(it) } },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Done),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        },
        confirmButton = {
            TextButton(onClick = lookUp, enabled = text.isNotBlank() && !busy) {
                if (busy) CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp) else Text(stringResource(R.string.address_continue))
            }
        },
        dismissButton = { TextButton(onClick = onDismiss, enabled = !busy) { Text(stringResource(R.string.cancel)) } },
    )
}
