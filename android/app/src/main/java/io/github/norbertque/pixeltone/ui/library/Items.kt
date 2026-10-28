package io.github.norbertque.pixeltone.ui.library

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.library.Album
import io.github.norbertque.pixeltone.library.AlbumCover
import io.github.norbertque.pixeltone.library.Artist
import io.github.norbertque.pixeltone.library.Track
import io.github.norbertque.pixeltone.ui.PixeltoneGlyph
import io.github.norbertque.pixeltone.ui.formatDuration

/** An album's cover, or Pixeltone's bars where it has none. */
@Composable
fun Cover(albumId: Long, modifier: Modifier = Modifier, shape: Shape = MaterialTheme.shapes.small) {
    Box(
        modifier.clip(shape).background(MaterialTheme.colorScheme.surfaceContainerHighest),
        contentAlignment = Alignment.Center,
    ) {
        PixeltoneGlyph(Modifier.fillMaxSize(0.4f), MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f))
        if (albumId >= 0) {
            AsyncImage(
                model = AlbumCover(albumId),
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.matchParentSize(),
            )
        }
    }
}

/** A track in a list: its cover, or its number on an album page. */
@Composable
fun TrackRow(
    track: Track,
    numbered: Boolean = false,
    showArtist: Boolean = true,
    showAlbum: Boolean = true,
    onClick: (() -> Unit)? = null,
) {
    val artist = track.artist ?: stringResource(R.string.unknown_artist)
    val details = listOfNotNull(artist.takeIf { showArtist }, track.album.takeIf { showAlbum }).joinToString(" · ")
    ListItem(
        headlineContent = { Text(track.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = if (details.isEmpty()) {
            null
        } else {
            { Text(details, maxLines = 1, overflow = TextOverflow.Ellipsis) }
        },
        leadingContent = {
            if (numbered) {
                Text(
                    track.trackNumber?.toString().orEmpty(),
                    modifier = Modifier.width(32.dp),
                    textAlign = TextAlign.End,
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            } else {
                Cover(track.albumId, Modifier.size(48.dp))
            }
        },
        trailingContent = { Text(formatDuration(track.durationMs), style = MaterialTheme.typography.labelMedium) },
        modifier = if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier,
    )
}

@Composable
fun AlbumCard(album: Album, onClick: () -> Unit, modifier: Modifier = Modifier) {
    Column(modifier.clip(MaterialTheme.shapes.medium).clickable(onClick = onClick)) {
        Cover(album.coverId, Modifier.fillMaxWidth().aspectRatio(1f), MaterialTheme.shapes.medium)
        Text(
            album.title,
            style = MaterialTheme.typography.titleSmall,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.padding(top = 8.dp, start = 2.dp, end = 2.dp),
        )
        Text(
            albumArtistName(album),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.padding(horizontal = 2.dp),
        )
    }
}

@Composable
fun ArtistRow(artist: Artist, onClick: () -> Unit) {
    val cover = artist.albums.firstOrNull()?.coverId ?: artist.tracks.firstOrNull()?.albumId ?: -1
    ListItem(
        headlineContent = { Text(artist.name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = { Text(counts(artist.albums.size, artist.tracks.size)) },
        leadingContent = { Cover(cover, Modifier.size(48.dp), CircleShape) },
        modifier = Modifier.clickable(onClick = onClick),
    )
}

@Composable
fun albumArtistName(album: Album): String =
    album.artist ?: stringResource(if (album.isCompilation) R.string.various_artists else R.string.unknown_artist)

@Composable
private fun counts(albums: Int, tracks: Int): String =
    listOfNotNull(
        pluralStringResource(R.plurals.albums_count, albums, albums).takeIf { albums > 0 },
        pluralStringResource(R.plurals.tracks_count, tracks, tracks).takeIf { tracks > 0 },
    ).joinToString(" · ")
