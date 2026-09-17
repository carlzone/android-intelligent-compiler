package dev.aic.host

import android.app.Activity
import android.app.Instrumentation
import android.content.Intent
import android.content.pm.PackageInstaller
import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.EditText
import org.json.JSONObject
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/** Runs real JNI/package/storage/lifecycle checks in the target application's UID. */
class HostInstrumentation : Instrumentation() {
    private val lines=mutableListOf<String>()
    override fun onCreate(arguments: Bundle?) { super.onCreate(arguments); start() }
    override fun onStart() {
        val output=Bundle()
        try {
            val context=targetContext
            check(context.assets.open("toolchain/NOTICE.txt").bufferedReader().use { it.readText() }.contains("Apache License"))
            check(context.assets.list("")?.contains("bootstrap") != true)
            context.getSharedPreferences("build",0).edit().putBoolean("running",true).putString("log","interrupted test").commit()
            val recovered=BuildController.get(context)
            check(!recovered.busy && recovered.log.contains("Previous build was interrupted"))
            lines+="PASS: process-death build checkpoint recovered with actionable retry diagnostic"
            val store=ProjectStore(context)
            fun acceptanceProject(data: ProjectData): StoredProject = store.list().firstOrNull { it.data.name == data.name }
                ?.let { store.save(it.copy(data=data)) } ?: store.create(data)
            val sample=context.assets.open("templates/hello.aic").bufferedReader().use { it.readText() }
            val unicode="\u7b46\u8a18 \ud83d\udcdd"
            val project=store.create(ProjectData("M6 Unicode $unicode",sample.replace("Hello from AndroidIntelligentCompiler",unicode),optLevel=0))
            check(store.load(project.id)==project)
            check(store.create(ProjectCodec.decode(ProjectCodec.encode(project.data))).data==project.data)
            check(runCatching { store.save(project.copy(data=project.data.copy(profile="invalid"))) }.isFailure)
            check(store.load(project.id)==project)
            lines+="PASS: private storage, Unicode, canonical archive round-trip, rejected save preserves project"

            val evidence=File(context.filesDir,"acceptance").apply { mkdirs() }
            for(name in listOf("hello","counter","calculator","notes","optimizer")) for(level in 0..1) {
                val source=context.assets.open("templates/$name.aic").bufferedReader().use { it.readText() }
                    .replace("dev.aic.generated.$name","dev.aic.m6.$name")
                val dir=File(evidence,"$name-o$level")
                val built=BuildPipeline(context).build(source,level,dir) { }
                File(dir,"source.aic").writeText(source)
                val repeat=File(dir,"repeat")
                val assets=File(repeat,"project-assets").apply { mkdirs() }
                check(JSONObject(NativeCompiler.compile(source,assets.path,repeat.path,level)).getBoolean("ok"))
                check(File(dir,"classes.dex").readBytes().contentEquals(File(repeat,"classes.dex").readBytes()))
                for(artifact in listOf("AndroidManifest.axml", "unsigned.apk")) {
                    check(File(dir,artifact).readBytes().contentEquals(File(repeat,artifact).readBytes()))
                }
                check(!File(dir,"tool-invocations.log").readText().contains("bundled:"))
                check(built.apk.length()>0 && built.packageName=="dev.aic.m6.$name")
                acceptanceProject(ProjectData("M6 $name O$level",source,optLevel=level))
                lines+="PASS: $name O$level JNI deterministic DEX, binary manifest, APK assembly, signing"
            }
            val emptyAssets=File(evidence,"empty-assets").apply { mkdirs() }
            val invalid=JSONObject(NativeCompiler.compile("invalid source",emptyAssets.path,File(evidence,"invalid").path,1))
            check(!invalid.getBoolean("ok") && invalid.getJSONArray("diagnostics").getJSONObject(0).getJSONObject("location").getInt("line")==1)
            val blocked=File(evidence,"not-a-directory").apply { writeText("occupied") }
            check(!JSONObject(NativeCompiler.compile(sample,emptyAssets.path,blocked.path,1)).getBoolean("ok"))
            lines+="PASS: source-located malformed-input errors and output storage failure"

            for(kind in ProviderKind.entries.filter { it.credentialLabel!=null }) {
                val secret="${kind.id}-test-not-a-real-key"; val previousSecret=SecretStore(context).credential(kind.id)
                SecretStore(context).saveCredential(kind.id,secret)
                check(SecretStore(context).credential(kind.id)==secret)
                check(!context.getSharedPreferences("ai-secrets",0).all.values.joinToString().contains(secret))
                SecretStore(context).saveCredential(kind.id,previousSecret)
            }
            lines+="PASS: per-provider credentials encrypted by Android Keystore and removable"

            val aiBase=context.assets.open("templates/counter.aic").bufferedReader().use { it.readText() }
            val aiPatched=aiBase.replace("AIC Counter","AIC Counter M7")
            var calls=0; var sawRepair=false
            val fake=ModelProvider { turn ->
                calls++; if(turn.diagnostics!=null) sawRepair=true
                val body=if(calls==1) """{"schema_version":"aic.model-proposal/1","operation":"patch","source":"aic_version 0.1\\napp \\"broken\\" package \\"dev.aic.generated.counter\\" { android.teleport() }","summary":"bad","touched_areas":["ui"]}"""
                else JSONObject().put("schema_version",AiProtocol.SCHEMA_VERSION).put("operation","patch").put("source",aiPatched)
                    .put("summary","Rename the app title").put("touched_areas",org.json.JSONArray().put("metadata")).toString()
                ModelAnswer("test","deterministic",null,body)
            }
            val accepted=AiRepairLoop(fake,{ JSONObject(NativeCompiler.validate(it,emptyAssets.path,1)) }).run(AiOperation.PATCH,"rename the title",aiBase)
            check(accepted.proposal?.source==aiPatched && accepted.attempts==2 && sawRepair)
            val failed=AiRepairLoop(ModelProvider { ModelAnswer("test","invalid",null,"{}") },{ JSONObject(NativeCompiler.validate(it,emptyAssets.path,1)) })
                .run(AiOperation.PATCH,"invent a teleport API",aiBase)
            check(failed.proposal==null && failed.diagnostic.contains("preserved") && calls==2)
            check(aiBase==context.assets.open("templates/counter.aic").bufferedReader().use { it.readText() })
            lines+="PASS: schema/semantic repair, bounded failure, and last-valid-source preservation"

            context.getSharedPreferences("editor",0).edit().putString("project",project.id).commit()
            val activity=startActivitySync(Intent(context,MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK))
            waitForIdleSync()
            val done=CountDownLatch(1)
            onMain {
                val editor=activity.findViewById<EditText>(R.id.source_editor)
                check(editor.text.toString()==project.data.source)
                editor.setText(project.data.source.replace(unicode, unicode+" edited"))
                check(editor.text.toString().contains(unicode+" edited"))
                descendants(activity.window.decorView).filterIsInstance<Button>().first { it.text=="Save" }.performClick()
                check(store.load(project.id).data.source.contains(unicode+" edited"))
                descendants(activity.window.decorView).filterIsInstance<Button>().first { it.text=="Build" }.performClick()
                check(BuildController.get(context).busy)
                activity.recreate()
                android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({ done.countDown() },1000)
            }
            check(done.await(10,TimeUnit.SECONDS))
            waitForIdleSync()
            val end=System.nanoTime()+TimeUnit.SECONDS.toNanos(60)
            while(System.nanoTime()<end) {
                var busy=true
                onMain { busy=BuildController.get(context).busy }
                if(!busy) break
                Thread.sleep(100)
            }
            onMain { check(BuildController.get(context).result != null) }
            lines+="PASS: Unicode editor save and Activity recreation while building"
            val installPrefs=context.getSharedPreferences("installer",0)
            installPrefs.edit().putInt("session",7306).commit()
            onMain {
                InstallReceiver().onReceive(context,Intent("dev.aic.host.INSTALL_STATUS")
                    .putExtra(PackageInstaller.EXTRA_SESSION_ID,7306)
                    .putExtra(PackageInstaller.EXTRA_STATUS,PackageInstaller.STATUS_FAILURE_CONFLICT)
                    .putExtra(PackageInstaller.EXTRA_STATUS_MESSAGE,"Existing package has a different signing identity"))
            }
            check(installPrefs.getString("status","")!!.contains("different signing identity"))
            check(installPrefs.getInt("session",0)==-1)
            lines+="PASS: installer failure/signing-conflict diagnostic persists across recreation"
            output.putString("stream",lines.joinToString("\n")+"\nM7_INSTRUMENTATION_PASS\n")
            File(evidence,"instrumentation.txt").writeText(output.getString("stream")!!)
            finish(Activity.RESULT_OK,output)
        } catch(e: Throwable) {
            output.putString("stream",lines.joinToString("\n")+"\nFAIL: "+android.util.Log.getStackTraceString(e))
            finish(Activity.RESULT_CANCELED,output)
        }
    }
    private fun onMain(action: () -> Unit) {
        var failure: Throwable? = null
        runOnMainSync { try { action() } catch (e: Throwable) { failure=e } }
        failure?.let { throw it }
    }
    private fun descendants(view: View): Sequence<View> = sequence {
        yield(view)
        if(view is ViewGroup) for(i in 0 until view.childCount) yieldAll(descendants(view.getChildAt(i)))
    }
}
