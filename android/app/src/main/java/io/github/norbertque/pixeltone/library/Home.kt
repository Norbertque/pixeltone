package io.github.norbertque.pixeltone.library

import kotlin.random.Random

internal const val DAY_MS = 86_400_000L

/**
 * Today's mix, by the Windows rule (`pp-library` daily_mix): half from the artists played most
 * in the last 60 days (tracks not heard in the last 2 days), a quarter favourites not heard for
 * a week, the rest from anywhere; the same all day, another tomorrow ([day] seeds it). A genre's
 * mix takes the same rule within its tracks ([pool]), seeded apart by [salt].
 */
fun dailyMix(
    index: LibraryIndex,
    listens: List<Listen>,
    favourites: List<ListedTrack>,
    day: Long,
    now: Long,
    limit: Int = 40,
    pool: List<Track> = index.tracks,
    salt: Int = 0,
): List<Track> {
    val random = Random((day * 0x9E3779B97F4A7C15uL.toLong() or 1) xor salt.toLong())
    val inPool = if (pool === index.tracks) null else pool.mapTo(HashSet(), Track::id)
    fun heardSince(since: Long) = listens.filter { it.startedAt >= since }.map { it.track.file }.toSet()
    val topArtists = listens.filter { it.startedAt >= now - 60 * DAY_MS }
        .flatMap { listen -> index.artistNames(listen.track.artist).map(::fold) }
        .groupingBy { it }.eachCount()
        .entries.sortedByDescending { it.value }.take(8).map { it.key }.toSet()
    val recent = heardSince(now - 2 * DAY_MS)
    val fromArtists = pool.filter { t -> index.artistNames(t.artist).any { fold(it) in topArtists } && t.fileKey !in recent }.shuffled(random)
    val week = heardSince(now - 7 * DAY_MS)
    val loved = favourites.filter { it.file !in week }
        .mapNotNull { index.trackByFile(it.file) }
        .filter { inPool == null || it.id in inPool }
        .shuffled(random)
    val anything = pool.shuffled(random)
    val chosen = LinkedHashMap<Long, Track>()
    fun take(pool: List<Track>, n: Int) {
        var taken = 0
        for (track in pool) {
            if (taken >= n || chosen.size >= limit) break
            if (chosen.putIfAbsent(track.id, track) == null) taken++
        }
    }
    take(fromArtists, limit / 2)
    take(loved, limit / 4)
    take(anything, limit)
    return chosen.values.toList().shuffled(random)
}

/** An album with its last listen (Home's "Recently played"). */
fun recentlyPlayedAlbums(index: LibraryIndex, listens: List<Listen>, limit: Int = 12): List<Album> {
    val albums = LinkedHashMap<String, Album>()
    for (listen in listens.asReversed()) {
        val track = index.trackByFile(listen.track.file) ?: continue
        val key = albumKey(track) ?: continue
        val album = index.album(key) ?: continue
        albums.putIfAbsent(key, album)
        if (albums.size >= limit) break
    }
    return albums.values.toList()
}

/** The artists listened to most since [since], with their plays (Home's "Your artists this month"). */
fun topArtists(index: LibraryIndex, listens: List<Listen>, since: Long, limit: Int = 10): List<Pair<Artist, Int>> =
    listens.filter { it.startedAt >= since }
        .flatMap { index.artistNames(it.track.artist) }
        .groupingBy(::fold).eachCount()
        .entries.sortedByDescending { it.value }
        .mapNotNull { (key, plays) -> index.artist(key)?.let { it to plays } }
        .take(limit)

/** Albums newest first by when their files came onto the phone (Home's "Recently added"). */
fun recentlyAdded(index: LibraryIndex, limit: Int = 12): List<Album> =
    index.albums.sortedByDescending { album -> album.tracks.maxOf { it.addedAt } }.take(limit)
