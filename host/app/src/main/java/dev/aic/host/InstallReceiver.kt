package dev.aic.host

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller

class InstallReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != "dev.aic.host.INSTALL_STATUS") return
        val prefs = context.getSharedPreferences("installer", Context.MODE_PRIVATE)
        if (intent.getIntExtra(PackageInstaller.EXTRA_SESSION_ID,-1) != prefs.getInt("session",-2)) return
        val status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS,PackageInstaller.STATUS_FAILURE)
        if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) {
            @Suppress("DEPRECATION")
            val approval = intent.getParcelableExtra<Intent>(Intent.EXTRA_INTENT)
            InstallState.approval = approval
            prefs.edit().putString("status","Waiting for Android installation confirmation").apply()
            // The foreground activity launches platform confirmation. Background builds
            // never attempt to open an installer without a visible user action.
        } else {
            InstallState.approval = null
            prefs.edit().putString("status",if(status == PackageInstaller.STATUS_SUCCESS) "Installed successfully" else
                "Installation failed ($status): ${intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE).orEmpty()}")
                .putBoolean("installed",status == PackageInstaller.STATUS_SUCCESS).putInt("session",-1).apply()
        }
        InstallState.changed?.invoke()
    }
}

object InstallState {
    var approval: Intent? = null
    var changed: (() -> Unit)? = null
}
