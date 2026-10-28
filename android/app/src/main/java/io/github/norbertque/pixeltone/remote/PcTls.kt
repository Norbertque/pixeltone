package io.github.norbertque.pixeltone.remote

import android.annotation.SuppressLint
import java.security.MessageDigest
import java.security.cert.CertificateException
import java.security.cert.X509Certificate
import javax.net.ssl.SSLContext
import javax.net.ssl.X509TrustManager
import okhttp3.OkHttpClient

/**
 * TLS to a Pixeltone PC. Its certificate is self-signed and made once per installation: the
 * phone remembers its SHA-256 fingerprint when pairing and refuses any other certificate
 * afterwards. Host names aren't checked; the pinned certificate is the PC's identity.
 */
internal object PcTls {
    fun fingerprint(cert: X509Certificate): String =
        MessageDigest.getInstance("SHA-256")
            .digest(cert.encoded)
            .joinToString("") { "%02x".format(it) }

    /** Talks only to the PC whose certificate has this fingerprint. */
    fun pinned(base: OkHttpClient, fingerprint: String): OkHttpClient {
        val expected = fingerprint.lowercase()
        return withTrustManager(
            base,
            CertificateCheck { cert ->
                if (fingerprint(cert) != expected) {
                    throw CertificateException("Not the paired PC: its certificate changed.")
                }
            },
        )
    }

    /**
     * First contact, for pairing: takes the PC's certificate as it comes and reports its
     * fingerprint, which must match the one the PC then names in its answer.
     */
    fun firstContact(base: OkHttpClient, seen: (String) -> Unit): OkHttpClient =
        withTrustManager(base, CertificateCheck { cert -> seen(fingerprint(cert)) })

    private fun withTrustManager(base: OkHttpClient, trustManager: X509TrustManager): OkHttpClient {
        val context = SSLContext.getInstance("TLS")
        context.init(null, arrayOf(trustManager), null)
        return base.newBuilder()
            .sslSocketFactory(context.socketFactory, trustManager)
            .hostnameVerifier { _, _ -> true }
            .build()
    }

    // Deliberately not the system's trust: a PC's certificate is its own, checked by its pinned fingerprint.
    @SuppressLint("CustomX509TrustManager")
    private class CertificateCheck(
        private val check: (X509Certificate) -> Unit,
    ) : X509TrustManager {
        override fun checkClientTrusted(chain: Array<out X509Certificate>?, authType: String?) {
            throw CertificateException("Client certificates aren't used.")
        }

        override fun checkServerTrusted(chain: Array<out X509Certificate>?, authType: String?) {
            check(chain?.firstOrNull() ?: throw CertificateException("The PC sent no certificate."))
        }

        override fun getAcceptedIssuers(): Array<X509Certificate> = emptyArray()
    }
}
