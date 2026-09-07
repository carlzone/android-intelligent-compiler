package dev.aic.host

import android.content.Context
import org.json.JSONObject
import java.io.File

data class BuiltApp(val apk: File, val packageName: String, val activity: String, val report: String)
class BuildFailure(val diagnostics: String) : Exception(diagnostics)

class BuildPipeline(private val context: Context) {
    fun build(source: String, level: Int, directory: File, progress: (String) -> Unit): BuiltApp {
        check(directory.mkdirs() || directory.isDirectory) { "Cannot create build directory" }
        val audit = File(directory, "tool-invocations.log")
        audit.writeText("AIC M8 generated-app build; profile=android-35; optLevel=$level\n")
        progress("Compiling, optimizing, and packaging AIC source")
        audit.appendText("embedded:aic-jni compile\n")
        val result = checked(NativeCompiler.compile(source, directory.path, level))
        audit.appendText("embedded:aic-res binary manifest; aic-build APK assembly (four-byte alignment)\n")
        progress("Signing APK with local debug identity")
        audit.appendText("embedded:AOSP apksig 8.10.1 (AndroidKeyStore, v1+v2)\n")
        val apk = File(directory,"signed.apk")
        try {
            DebugSigner.sign(File(directory,"unsigned.apk"), apk)
        } catch (e: Exception) {
            apk.delete()
            throw BuildFailure("AIC8003 sign: ${e.javaClass.simpleName}: APK signing failed; check local signing identity and available storage")
        }
        return BuiltApp(apk, result.getString("package"), result.getString("activity"), result.getString("report"))
    }

    private fun checked(json: String): JSONObject {
        val result = JSONObject(json)
        if (!result.getBoolean("ok")) {
            val diagnostics = result.getJSONArray("diagnostics")
            throw BuildFailure((0 until diagnostics.length()).joinToString("\n") {
                val d = diagnostics.getJSONObject(it)
                val loc = d.optJSONObject("location")?.let { p -> " at ${p.getInt("line")}:${p.getInt("column")}" }.orEmpty()
                "${d.getString("code")} ${d.getString("stage")}$loc: ${d.getString("message")}"
            })
        }
        return result
    }
}
