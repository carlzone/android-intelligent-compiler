package dev.aic.host

import android.content.Context
import android.os.Handler
import android.os.Looper
import org.json.JSONObject
import java.util.concurrent.Executors
import java.io.File
import java.util.UUID

class AiController private constructor(private val context: Context) {
    private val worker=Executors.newSingleThreadExecutor(); private val main=Handler(Looper.getMainLooper())
    var busy=false; private set
    var outcome: AiOutcome?=null; private set
    var log="AI ready. Configure an API key, then describe a new app or an edit."; private set
    var changed: (() -> Unit)?=null

    fun start(projectId: String?, operation: AiOperation, prompt: String, current: String?, level: Int, images: Map<String,ByteArray>) {
        check(!busy) { "An AI request is already running" }
        val prefs=context.getSharedPreferences("ai-settings",Context.MODE_PRIVATE)
        val kind=ProviderKind.from(prefs.getString("provider",ProviderKind.OPENAI.id))
        val model=prefs.getString("model.${kind.id}",kind.defaultModel).orEmpty().ifBlank { kind.defaultModel }
        val baseUrl=prefs.getString("url.${kind.id}",kind.defaultUrl)
        val config=ProviderConfig(kind,model,baseUrl,SecretStore(context).credential(kind.id))
        val provider=ModelProviderFactory.create(context,config)
        val provenance=AiProvenance(context)
        busy=true; outcome=null; log="AI ${operation.wire} request running…"; changed?.invoke()
        worker.execute {
            val assetDir=File(context.cacheDir,"ai-assets/${UUID.randomUUID()}").apply { mkdirs() }
            for((name,bytes) in images.toSortedMap()) File(assetDir,name).writeBytes(bytes)
            val result=runCatching {
                AiRepairLoop(provider,{ JSONObject(NativeCompiler.validate(it,assetDir.path,level)) }) { turn,answer,status ->
                    provenance.record(projectId,turn,answer,status)
                }.run(operation,prompt,current)
            }
            assetDir.deleteRecursively()
            main.post {
                busy=false
                result.fold({ value -> outcome=value; log=value.diagnostic }, { error ->
                    outcome=null; log="AIC7099 ai: ${error.message ?: error.javaClass.simpleName}. Last valid source was preserved."
                })
                changed?.invoke()
            }
        }
    }

    fun clear() { outcome=null; changed?.invoke() }
    companion object {
        const val DEFAULT_MODEL="gpt-5.4-mini-2026-03-17"
        @Volatile private var instance: AiController?=null
        fun get(context: Context): AiController=instance ?: synchronized(this) {
            instance ?: AiController(context.applicationContext).also { instance=it }
        }
    }
}
