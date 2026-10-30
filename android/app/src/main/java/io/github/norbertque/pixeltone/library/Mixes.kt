package io.github.norbertque.pixeltone.library

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.flow.map

/** A genre has a mix of its own from this many tracks on. */
const val MIX_MIN_TRACKS = 10

/** At most this many genres get a mix. */
const val MAX_GENRE_MIXES = 12

/** A playlist the library makes by itself every day: today's mix, or a genre's. */
data class Mix(
    /** [DAILY], or "genre:" and the genre's key. */
    val key: String,
    /** The genre it's made from; null for today's mix. */
    val genre: Genre?,
    val tracks: List<Track>,
) {
    companion object {
        const val DAILY = "daily"

        fun keyOf(genre: Genre) = "genre:${genre.key}"
    }
}

/** Today's mix, and the genres' mixes. */
data class Mixes(val daily: Mix, val genres: List<Mix>) {
    fun mix(key: String): Mix? = if (key == daily.key) daily else genres.firstOrNull { it.key == key }
}

/**
 * The genres worth a mix: those with [MIX_MIN_TRACKS] tracks, the ones listened to most in the
 * last 60 days first, then the biggest.
 */
fun mixGenres(index: LibraryIndex, listens: List<Listen>, now: Long): List<Genre> {
    val plays = HashMap<String, Int>()
    for (listen in listens) {
        if (listen.startedAt < now - 60 * DAY_MS) continue
        val track = index.trackByFile(listen.track.file) ?: continue
        for (genre in track.genres) plays.merge(fold(genre), 1) { a, b -> a + b }
    }
    return index.genres
        .filter { it.tracks.size >= MIX_MIN_TRACKS }
        .sortedWith(compareByDescending<Genre> { plays[it.key] ?: 0 }.thenByDescending { it.tracks.size })
        .take(MAX_GENRE_MIXES)
}

/**
 * Today's mixes ([dailyMix], and its rule within each of [mixGenres]). Only what was heard before
 * [day] counts, so they stay the same all day while today's listening goes on.
 */
fun todaysMixes(index: LibraryIndex, listens: List<Listen>, favourites: List<ListedTrack>, day: Long): Mixes {
    val start = day * DAY_MS
    val before = listens.filter { it.startedAt < start }
    return Mixes(
        daily = Mix(Mix.DAILY, null, dailyMix(index, before, favourites, day, now = start)),
        genres = mixGenres(index, before, now = start).map { genre ->
            Mix(Mix.keyOf(genre), genre, dailyMix(index, before, favourites, day, now = start, pool = genre.tracks, salt = genre.key.hashCode()))
        },
    )
}

/** [todaysMixes], made again when the library, the history or the favourites change, or a day begins. */
fun mixesOf(
    index: Flow<LibraryIndex?>,
    listens: Flow<List<Listen>>,
    lists: Flow<Lists>,
    clock: () -> Long = System::currentTimeMillis,
): Flow<Mixes?> {
    val days = flow {
        while (true) {
            emit(clock() / DAY_MS)
            delay(60_000)
        }
    }.distinctUntilChanged()
    val favourites = lists.map { it.favourites }.distinctUntilChanged()
    return combine(index, listens, favourites, days) { library, heard, loved, day ->
        library?.let { todaysMixes(it, heard, loved, day) }
    }.distinctUntilChanged().flowOn(Dispatchers.Default)
}
