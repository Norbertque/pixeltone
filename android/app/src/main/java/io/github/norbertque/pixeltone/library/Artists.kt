package io.github.norbertque.pixeltone.library

/**
 * What an artist tag is split on by default (ignoring case), as on Windows (`DEFAULT_DELIMITERS`).
 * "&" and "," aren't among them, so "Simon & Garfunkel" stays whole; they can be added in Settings.
 */
val DEFAULT_ARTIST_SEPARATORS: List<String> = listOf(
    ";",
    "/",
    " feat. ",
    " ft. ",
    " featuring ",
    " feat ",
    " ft ",
    "(feat. ",
    "(ft. ",
    "[feat. ",
    "[ft. ",
)

/** Names with a separator in them that are one artist. */
private val PROTECTED_NAMES = listOf("AC/DC")

/** An artist tag split into its artists, each once, in order, as on Windows (`split_artists`). */
fun splitArtists(value: String, separators: List<String> = DEFAULT_ARTIST_SEPARATORS): List<String> {
    var protectedValue = value
    PROTECTED_NAMES.forEachIndexed { i, name -> protectedValue = protectedValue.replace(name, placeholder(i), ignoreCase = true) }
    var parts = listOf(protectedValue)
    for (separator in separators.filter(String::isNotEmpty)) {
        parts = parts.flatMap { it.split(separator, ignoreCase = true) }
    }
    val seen = HashSet<String>()
    return parts
        .map { part ->
            var name = part.trim { it.isWhitespace() || it in "()[]" }
            PROTECTED_NAMES.forEachIndexed { i, protected -> name = name.replace(placeholder(i), protected) }
            name
        }
        .filter { it.isNotEmpty() && seen.add(fold(it)) }
}

private fun placeholder(i: Int) = "\u0001$i\u0001"
