package io.github.norbertque.pixeltone.remote

import java.net.URLDecoder

/**
 * A jam's QR code or link: `https://norbertque.github.io/pixeltone/jam/#v=1&h=…&p=…&f=…&s=…&n=…`
 * (the PC's address, its certificate's fingerprint, the jam's secret and the PC's name; all
 * after `#`, so browsers never send them anywhere). The page's "Open in Pixeltone" button
 * sends the same as `pixeltone://jam?…`.
 */
data class JamInvite(
    val host: String,
    val port: Int,
    val fingerprint: String,
    val secret: String,
    val pcName: String,
)

const val JAM_PAGE = "https://norbertque.github.io/pixeltone/jam/"

/** The invite in a jam link, or null if it isn't one this app understands. */
fun parseJamLink(link: String): JamInvite? {
    val text = link.trim()
    val params = when {
        text.startsWith(JAM_PAGE.removeSuffix("/")) -> text.substringAfter('#', "")
        text.startsWith("pixeltone://jam") -> text.substringAfter('?', "").ifEmpty { text.substringAfter('#', "") }
        else -> return null
    }
    val values = params.split('&').mapNotNull { pair ->
        val key = pair.substringBefore('=', "")
        if (key.isEmpty()) null else key to runCatching { URLDecoder.decode(pair.substringAfter('='), "UTF-8") }.getOrNull()
    }.toMap()
    if (values["v"]?.toIntOrNull() != RemoteProtocol.VERSION) return null
    val host = values["h"]?.takeIf { it.isNotBlank() } ?: return null
    val port = values["p"]?.toIntOrNull()?.takeIf { it in 1..65_535 } ?: return null
    val fingerprint = values["f"]?.lowercase()?.takeIf { f -> f.length == 64 && f.all { it in "0123456789abcdef" } } ?: return null
    val secret = values["s"]?.takeIf { it.isNotBlank() } ?: return null
    return JamInvite(host, port, fingerprint, secret, values["n"]?.takeIf { it.isNotBlank() } ?: host)
}
