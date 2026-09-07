package dev.aic.host

import android.app.Activity
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.net.Uri
import android.provider.Settings

class AppInstaller(private val context: Context) {
    val prefs = context.getSharedPreferences("installer",Context.MODE_PRIVATE)
    @android.annotation.SuppressLint("ApplySharedPref") // Session identity must survive process death before commit.
    fun install(app: BuiltApp) {
        check(context.packageManager.canRequestPackageInstalls()) { "Allow AIC Host to install apps, then tap Install again" }
        val active = prefs.getInt("session",-1)
        check(active == -1 || context.packageManager.packageInstaller.getSessionInfo(active) == null) { "An installation is already pending" }
        val installer = context.packageManager.packageInstaller
        val params = PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL).apply {
            setAppPackageName(app.packageName)
            setSize(app.apk.length())
            if (android.os.Build.VERSION.SDK_INT >= 31) setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_REQUIRED)
        }
        val id = installer.createSession(params)
        try {
            check(prefs.edit().putInt("session",id).putString("package",app.packageName).putString("activity",app.activity)
                .putString("status","Submitting APK to Android").putBoolean("installed",false).commit()) { "Cannot save installation state; check available storage" }
            installer.openSession(id).use { session ->
                session.openWrite("base.apk",0,app.apk.length()).use { out -> app.apk.inputStream().use { it.copyTo(out) }; session.fsync(out) }
                val intent = Intent(context,InstallReceiver::class.java).setAction("dev.aic.host.INSTALL_STATUS")
                val flags = PendingIntent.FLAG_UPDATE_CURRENT or if(android.os.Build.VERSION.SDK_INT >= 31) PendingIntent.FLAG_MUTABLE else 0
                session.commit(PendingIntent.getBroadcast(context,id,intent,flags).intentSender)
            }
        } catch (e: Exception) {
            installer.abandonSession(id)
            prefs.edit().putInt("session",-1).putString("status","Installation submission failed: ${e.message}").apply()
            throw e
        }
    }
    fun allowSource(activity: Activity) {
        activity.startActivity(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,Uri.parse("package:${context.packageName}")))
    }
    fun cancel() {
        val id = prefs.getInt("session",-1)
        if (id >= 0) runCatching { context.packageManager.packageInstaller.abandonSession(id) }
        InstallState.approval = null
        prefs.edit().putInt("session",-1).putString("status","Installation cancelled").apply()
    }
    fun launch(activity: Activity) {
        check(prefs.getBoolean("installed",false)) { "Install a generated app first" }
        activity.startActivity(Intent().setClassName(prefs.getString("package","")!!,prefs.getString("activity","")!!)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }
}
