package io.github.norbertque.pixeltone.library

/** Something listened to, with how often and how long. */
data class Ranked<T>(val item: T, val plays: Int, val heardMs: Long)

data class AlbumName(val title: String, val artist: String?)

data class ListeningStats(
    val heardMs: Long = 0,
    val plays: Int = 0,
    val tracks: List<Ranked<ListedTrack>> = emptyList(),
    val artists: List<Ranked<String>> = emptyList(),
    val albums: List<Ranked<AlbumName>> = emptyList(),
    /** Newest first. */
    val recent: List<Listen> = emptyList(),
)

/**
 * The listens since [since] (epoch ms; null: all time), summed up: the most played tracks,
 * artists and albums, [top] of each. Names are matched ignoring case and accents.
 */
fun listeningStats(listens: List<Listen>, since: Long?, top: Int = 10): ListeningStats {
    val shown = if (since == null) listens else listens.filter { it.startedAt >= since }
    fun <K, T> rank(key: (Listen) -> K?, item: (List<Listen>) -> T): List<Ranked<T>> =
        shown.groupBy { key(it) }
            .filterKeys { it != null }
            .map { (_, group) -> Ranked(item(group), group.size, group.sumOf(Listen::heardMs)) }
            .sortedWith(compareByDescending<Ranked<T>> { it.plays }.thenByDescending { it.heardMs })
            .take(top)
    return ListeningStats(
        heardMs = shown.sumOf(Listen::heardMs),
        plays = shown.size,
        tracks = rank({ it.track.file }) { it.last().track },
        artists = rank({ it.track.artist?.let(::fold) }) { group -> mostCommon(group.mapNotNull { it.track.artist }) },
        albums = rank({ l -> l.track.album?.let { fold(it) + "|" + fold(l.track.artist.orEmpty()) } }) { group ->
            AlbumName(mostCommon(group.mapNotNull { it.track.album }), group.firstNotNullOfOrNull { it.track.artist })
        },
        recent = shown.asReversed().take(RECENT),
    )
}

private const val RECENT = 30

private fun mostCommon(names: List<String>): String = names.groupingBy { it }.eachCount().maxBy { it.value }.key
