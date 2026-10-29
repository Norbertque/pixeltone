package io.github.norbertque.pixeltone.lyrics

/** A line of lyrics; [timeMs] is null for plain (unsynced) lyrics. */
data class LyricLine(val timeMs: Long?, val text: String)

/** `mm:ss`, `mm:ss.x…`, `mm:ss:xx` (some taggers); minutes may go past 59. */
private fun parseTime(tag: String): Long? {
    val min = tag.substringBefore(':', "").trim().toLongOrNull() ?: return null
    val rest = tag.substringAfter(':')
    val cut = rest.indexOfAny(charArrayOf('.', ':'))
    val sec = (if (cut >= 0) rest.substring(0, cut) else rest).trim().toLongOrNull() ?: return null
    val frac = if (cut >= 0) rest.substring(cut + 1) else ""
    if (sec >= 60 || !frac.all(Char::isDigit)) return null
    // Fraction digits: 1 → tenths, 2 → hundredths, 3 → ms.
    val fracMs = when (frac.length) {
        0 -> 0L
        1 -> frac.toLong() * 100
        2 -> frac.toLong() * 10
        else -> frac.substring(0, 3).toLong()
    }
    return (min * 60 + sec) * 1000 + fracMs
}

/** Removes enhanced-LRC word stamps like `<01:02.03>`. */
private fun stripWordStamps(text: String): String {
    val out = StringBuilder(text.length)
    var rest = text
    while (true) {
        val start = rest.indexOf('<')
        if (start < 0) break
        val len = rest.indexOf('>', start)
        if (len > start && parseTime(rest.substring(start + 1, len)) != null) {
            out.append(rest, 0, start)
            rest = rest.substring(len + 1)
        } else {
            out.append(rest, 0, start + 1)
            rest = rest.substring(start + 1)
        }
    }
    out.append(rest)
    return out.toString().trim().split(Regex("\\s+")).filter(String::isNotEmpty).joinToString(" ")
}

/**
 * LRC, as on Windows (`crates/pp-lyrics/src/lrc.rs`): `[mm:ss.xx]` timed lines (several stamps
 * per line), `[offset:±ms]`, enhanced `<mm:ss.xx>` word stamps (dropped), metadata tags
 * (ignored). Text without any timestamps is plain lyrics. Returns the lines and whether they're
 * synced.
 */
fun parseLrc(text: String): Pair<List<LyricLine>, Boolean> {
    val timed = ArrayList<LyricLine>()
    val plain = ArrayList<LyricLine>()
    var offset = 0L
    for (raw in text.lines()) {
        var line = raw.trim()
        val times = ArrayList<Long>()
        var tagOnly = false
        while (line.startsWith('[')) {
            val end = line.indexOf(']')
            if (end < 0) break
            val tag = line.substring(1, end)
            val time = parseTime(tag)
            if (time != null) {
                times += time
            } else if (':' in tag) {
                if (tag.substringBefore(':').trim().equals("offset", ignoreCase = true)) {
                    offset = tag.substringAfter(':').trim().toLongOrNull() ?: 0L
                }
                tagOnly = true
            } else {
                break
            }
            line = line.substring(end + 1).trimStart()
        }
        when {
            times.isNotEmpty() -> {
                val words = stripWordStamps(line)
                times.forEach { timed += LyricLine(it, words) }
            }
            !tagOnly -> plain += LyricLine(null, raw.trim())
        }
    }
    if (timed.isEmpty()) {
        // Blank lines at the ends of plain lyrics go.
        val first = plain.indexOfFirst { it.text.isNotEmpty() }
        val last = plain.indexOfLast { it.text.isNotEmpty() }
        return (if (first < 0) emptyList() else plain.subList(first, last + 1).toList()) to false
    }
    // A positive offset makes lyrics appear earlier.
    return timed.map { it.copy(timeMs = (it.timeMs!! - offset).coerceAtLeast(0)) }.sortedBy { it.timeMs } to true
}
