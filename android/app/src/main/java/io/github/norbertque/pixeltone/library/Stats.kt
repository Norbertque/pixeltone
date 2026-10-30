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
 * artists and albums, [top] of each. Names are matched ignoring case and accents; [artistsOf]
 * splits an artist tag into its artists (each gets the listen).
 */
fun listeningStats(
    listens: List<Listen>,
    since: Long?,
    top: Int = 10,
    artistsOf: (String) -> List<String> = { listOf(it) },
): ListeningStats {
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
        artists = shown
            .flatMap { listen -> listen.track.artist?.let(artistsOf).orEmpty().map { it to listen } }
            .groupBy { fold(it.first) }
            .map { (_, heard) -> Ranked(mostCommon(heard.map { it.first }), heard.size, heard.sumOf { it.second.heardMs }) }
            .sortedWith(compareByDescending<Ranked<String>> { it.plays }.thenByDescending { it.heardMs })
            .take(top),
        // An album is its title and its first artist: "A" and "A feat. B" share it.
        albums = rank({ l -> l.track.album?.let { fold(it) + "|" + fold(l.track.artist?.let(artistsOf)?.firstOrNull().orEmpty()) } }) { group ->
            AlbumName(mostCommon(group.mapNotNull { it.track.album }), group.firstNotNullOfOrNull { it.track.artist?.let(artistsOf)?.firstOrNull() })
        },
        recent = shown.asReversed().take(RECENT),
    )
}

private const val RECENT = 30

private fun mostCommon(names: List<String>): String = names.groupingBy { it }.eachCount().maxBy { it.value }.key
