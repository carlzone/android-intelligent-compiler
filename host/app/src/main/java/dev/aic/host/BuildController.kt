package dev.aic.host

import android.content.Context
import android.os.Handler
import android.os.Looper
import org.json.JSONObject
import java.io.File
import java.security.MessageDigest
import java.util.UUID
import java.util.concurrent.Executors

/** Process-owned job survives Activity recreation; persistent state detects process death. */
class BuildController private constructor(private val context: Context) {
    private val worker=Executors.newSingleThreadExecutor()
    private val main=Handler(Looper.getMainLooper())
    private val prefs=context.getSharedPreferences("build",Context.MODE_PRIVATE)
    var busy=false; private set
    var projectId=prefs.getString("project","").orEmpty(); private set
    var digest=prefs.getString("digest","").orEmpty(); private set
    var log=prefs.getString("log","Ready. Create or import an AIC project.").orEmpty(); private set
    var result: BuiltApp?=null; private set
    var changed: (() -> Unit)?=null
    init {
        if(prefs.getBoolean("running",false)) {
            log="AIC6103 build: Previous build was interrupted. Source is saved; tap Build to retry."
            prefs.edit().putBoolean("running",false).putString("log",log).remove("result").apply()
        } else {
            result=runCatching {
                val data=JSONObject(prefs.getString("result","").orEmpty())
                val file=File(context.filesDir,data.getString("apk"))
                check(file.canonicalPath.startsWith(File(context.filesDir,"builds").canonicalPath+File.separator) && file.isFile)
                BuiltApp(file,data.getString("package"),data.getString("activity"),data.getString("report"))
            }.getOrNull()
        }
    }
    @android.annotation.SuppressLint("ApplySharedPref") // Durable checkpoint before launching background work.
    fun start(project: StoredProject) {
        check(!busy) { "A build is already running" }
        busy=true; result=null; projectId=project.id; digest=hash(project.data)
        log="Building ${project.data.name} at O${project.data.optLevel}"
        if (!prefs.edit().putBoolean("running",true).putString("project",projectId).putString("digest",digest).putString("log",log).remove("result").commit()) {
            busy=false
            error("Cannot save build state; check available storage")
        }
        changed?.invoke()
        worker.execute {
            val directory=File(context.filesDir,"builds/${UUID.randomUUID()}")
            val outcome=runCatching { BuildPipeline(context).build(project.data.source,project.data.optLevel,directory) { message -> main.post { append(message) } } }
            main.post {
                busy=false
                outcome.fold({ built ->
                    result=built
                    append("Build succeeded: ${built.apk.length()} bytes\n${built.report}")
                    prefs.edit().putString("result",JSONObject().put("apk",built.apk.relativeTo(context.filesDir).path)
                        .put("package",built.packageName).put("activity",built.activity).put("report",built.report).toString()).apply()
                }, { error -> append(if(error is BuildFailure) error.diagnostics else "AIC6199 build: ${error.message ?: error.javaClass.simpleName}") })
                if (!prefs.edit().putBoolean("running",false).commit()) {
                    result=null
                    append("AIC6104 storage: Cannot commit build state; check available storage and rebuild")
                }
                changed?.invoke()
            }
        }
    }
    private fun append(message: String) { log=(log+"\n"+message).takeLast(65536); prefs.edit().putString("log",log).apply(); changed?.invoke() }
    companion object {
        @Volatile private var instance: BuildController?=null
        fun get(context: Context): BuildController = instance ?: synchronized(this) {
            instance ?: BuildController(context.applicationContext).also { instance=it }
        }
        fun hash(data: ProjectData): String = MessageDigest.getInstance("SHA-256")
            .digest((data.profile+"\n"+data.optLevel+"\n"+data.source).toByteArray(Charsets.UTF_8)).joinToString("") { "%02x".format(it) }
    }
}
