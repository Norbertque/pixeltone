package io.github.norbertque.pixeltone.ui.settings

import androidx.annotation.StringRes
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Computer
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.LibraryMusic
import androidx.compose.material.icons.filled.Lyrics
import androidx.compose.material.icons.filled.Palette
import androidx.compose.material.icons.filled.PlayCircle
import androidx.compose.material.icons.filled.SystemUpdate
import androidx.compose.material.icons.filled.Usb
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.ui.BackButton
import io.github.norbertque.pixeltone.ui.theme.Emphasized

/** Settings' parts, a page each. */
enum class SettingsPage(@StringRes val title: Int, @StringRes val summary: Int, val icon: ImageVector) {
    Appearance(R.string.appearance, R.string.appearance_summary, Icons.Default.Palette),
    Playback(R.string.playback, R.string.playback_summary, Icons.Default.PlayCircle),
    Library(R.string.library, R.string.library_summary, Icons.Default.LibraryMusic),
    Lyrics(R.string.lyrics, R.string.lyrics_summary, Icons.Default.Lyrics),
    Usb(R.string.usb_title, R.string.usb_summary, Icons.Default.Usb),
    Pcs(R.string.your_pcs, R.string.your_pcs_summary, Icons.Default.Computer),
    Updates(R.string.updates, R.string.updates_summary, Icons.Default.SystemUpdate),
    About(R.string.about, R.string.about_summary, Icons.Default.Info),
}

/** Settings' first page: its parts. */
@Composable
fun SettingsHome(onPage: (SettingsPage) -> Unit, onBack: () -> Unit) {
    SettingsScaffold(stringResource(R.string.settings), onBack) {
        items(SettingsPage.entries) { page ->
            ListItem(
                headlineContent = { Text(stringResource(page.title)) },
                supportingContent = { Text(stringResource(page.summary), maxLines = 2, overflow = TextOverflow.Ellipsis) },
                leadingContent = {
                    Box(
                        Modifier.size(40.dp).clip(CircleShape).background(MaterialTheme.colorScheme.secondaryContainer),
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(page.icon, null, Modifier.size(22.dp), tint = MaterialTheme.colorScheme.onSecondaryContainer)
                    }
                },
                trailingContent = { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null) },
                modifier = Modifier.clickable { onPage(page) },
            )
        }
    }
}

/** A page of settings: a big title that shrinks once the page scrolls, then [content]. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScaffold(title: String, onBack: () -> Unit, content: LazyListScope.() -> Unit) {
    val scroll = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()
    Scaffold(
        modifier = Modifier.nestedScroll(scroll.nestedScrollConnection),
        topBar = {
            LargeTopAppBar(
                title = { Text(title, fontWeight = Emphasized, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = { BackButton(onBack) },
                scrollBehavior = scroll,
            )
        },
    ) { padding ->
        LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(top = padding.calculateTopPadding(), bottom = 24.dp), content = content)
    }
}

@Composable
internal fun SectionTitle(text: String) {
    Text(
        text,
        Modifier.padding(start = 16.dp, end = 16.dp, top = 20.dp, bottom = 4.dp),
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
    )
}

/** A setting that's on or off; the whole row switches it. */
@Composable
internal fun SwitchRow(title: String, text: String?, checked: Boolean, onChange: (Boolean) -> Unit) {
    ListItem(
        headlineContent = { Text(title) },
        supportingContent = text?.let { { Text(it) } },
        trailingContent = { Switch(checked = checked, onCheckedChange = null) },
        modifier = Modifier.toggleable(checked, role = Role.Switch, onValueChange = onChange),
    )
}

/** A setting with a few choices, side by side. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun <T> ChoiceRow(title: String, text: String?, options: List<Pair<T, String>>, value: T, onChange: (T) -> Unit) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp)) {
        Text(title, style = MaterialTheme.typography.bodyLarge)
        if (text != null) {
            Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth().padding(top = 12.dp)) {
            options.forEachIndexed { i, (option, label) ->
                SegmentedButton(
                    selected = option == value,
                    onClick = { onChange(option) },
                    shape = SegmentedButtonDefaults.itemShape(i, options.size),
                ) {
                    Text(label, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
        }
    }
}
