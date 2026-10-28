package io.github.norbertque.pixeltone.update

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import androidx.core.content.IntentCompat
import io.github.norbertque.pixeltone.PixeltoneApp

/** Hears from Android's installer: shows its confirmation, then reports how it went. */
class InstallReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val updater = (context.applicationContext as PixeltoneApp).updater
        when (val status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)) {
            PackageInstaller.STATUS_PENDING_USER_ACTION -> {
                val confirm = IntentCompat.getParcelableExtra(intent, Intent.EXTRA_INTENT, Intent::class.java)
                if (confirm != null) {
                    context.startActivity(confirm.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                } else {
                    updater.installFinished("Android didn't ask to confirm the update.")
                }
            }
            // Success restarts the app as the new version; a cancelled confirmation is no error.
            PackageInstaller.STATUS_SUCCESS, PackageInstaller.STATUS_FAILURE_ABORTED -> updater.installFinished(null)
            else -> updater.installFinished(intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE) ?: "Status $status")
        }
    }
}
