package io.github.norbertque.pixeltone.library

import java.text.Collator
import java.util.Locale

/** The orders of the Tracks tab, as on Windows (`TrackSort`). */
enum class TrackSort { Title, Artist, Album, Year, Added, Duration }

/** The orders of the Albums tab, as on Windows (`AlbumSort`). */
enum class AlbumSort { Title, Artist, Year, Added }

/** [tracks] (by title, as the library has them) in [sort]'s order, as Windows' library orders them. */
fun sortTracks(tracks: List<Track>, sort: TrackSort, locale: Locale = Locale.getDefault()): List<Track> {
    val collator = Collator.getInstance(locale).apply { strength = Collator.SECONDARY }
    val byText: Comparator<String?> = nullsLast(collator)
    val inAlbum = compareBy<Track>({ it.discNumber ?: 1 }, { it.trackNumber ?: 0 })
    return when (sort) {
        TrackSort.Title -> tracks
        TrackSort.Artist -> tracks.sortedWith(compareBy<Track, String?>(byText) { it.artist }.thenBy(byText) { it.album }.then(inAlbum))
        TrackSort.Album -> tracks.sortedWith(compareBy<Track, String?>(byText) { it.album }.thenBy { albumKey(it) }.then(inAlbum))
        TrackSort.Year -> tracks.sortedWith(compareByDescending<Track, Int?>(nullsFirst()) { it.year }.thenBy(byText) { it.album }.then(inAlbum))
        TrackSort.Added -> tracks.sortedWith(compareByDescending<Track> { it.addedAt }.thenByDescending { it.id })
        TrackSort.Duration -> tracks.sortedByDescending { it.durationMs }
    }
}

/** [albums] (by title, as the library has them) in [sort]'s order, as Windows' library orders them. */
fun sortAlbums(albums: List<Album>, sort: AlbumSort, locale: Locale = Locale.getDefault()): List<Album> {
    val collator = Collator.getInstance(locale).apply { strength = Collator.SECONDARY }
    val byText: Comparator<String?> = nullsLast(collator)
    return when (sort) {
        AlbumSort.Title -> albums
        AlbumSort.Artist -> albums.sortedWith(compareBy<Album, String?>(byText) { it.artist }.thenBy { it.year ?: Int.MAX_VALUE }.thenBy(byText) { it.title })
        AlbumSort.Year -> albums.sortedWith(compareByDescending<Album, Int?>(nullsFirst()) { it.year }.thenBy(byText) { it.title })
        AlbumSort.Added -> albums.sortedByDescending { album -> album.tracks.maxOf { it.addedAt } }
    }
}
