package dev.aic.host

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URI
import java.net.URL

enum class ProviderKind(val id: String, val label: String, val defaultModel: String, val defaultUrl: String?, val credentialLabel: String?, val credentialRequired: Boolean=false) {
    OPENAI("openai","OpenAI","gpt-5.4-mini-2026-03-17",null,"API key",true),
    CLAUDE("claude","Claude","claude-sonnet-4-5",null,"API key",true),
    OPENROUTER("openrouter","OpenRouter","anthropic/claude-sonnet-4.5",null,"API key",true),
    HUGGING_FACE("huggingface","Hugging Face","openai/gpt-oss-120b:fastest",null,"HF token",true),
    OLLAMA("ollama","Ollama","llama3.2:latest","http://127.0.0.1:11434",null),
    LLAMA_CPP("llamacpp","llama.cpp","qwen3-8b","http://127.0.0.1:8080","API token (optional)");
    companion object { fun from(id: String?)=entries.firstOrNull { it.id==id } ?: OPENAI }
}

data class ProviderConfig(val kind: ProviderKind, val model: String, val baseUrl: String?, val credential: String)

object ModelProviderFactory {
    fun create(context: Context, config: ProviderConfig): ModelProvider=when(config.kind) {
        ProviderKind.OPENAI -> OpenAiProvider(context,config.credential,config.model)
        ProviderKind.CLAUDE -> ClaudeProvider(context,config.credential,config.model)
        ProviderKind.OPENROUTER -> ChatCompletionsProvider(context,config,"https://openrouter.ai/api/v1/chat/completions")
        ProviderKind.HUGGING_FACE -> ChatCompletionsProvider(context,config,"https://router.huggingface.co/v1/chat/completions")
        ProviderKind.OLLAMA -> OllamaProvider(context,config)
        ProviderKind.LLAMA_CPP -> ChatCompletionsProvider(context,config,localEndpoint(config.baseUrl,"/v1/chat/completions"))
    }
    fun localEndpoint(base: String?, path: String): String {
        val raw=base?.trim()?.trimEnd('/') ?: error("Local server URL is required")
        val uri=URI(raw); require(uri.scheme in setOf("http","https") && !uri.host.isNullOrBlank() && uri.userInfo==null && uri.query==null && uri.fragment==null) { "Invalid local server URL" }
        if(uri.scheme=="http") require(isPrivateHost(uri.host)) { "Plain HTTP is allowed only for a loopback or private LAN address" }
        return raw+path
    }
    private fun isPrivateHost(host: String): Boolean {
        val h=host.lowercase(); if(h=="localhost" || h=="10.0.2.2" || h.startsWith("127.")) return true
        val p=h.split('.').mapNotNull { it.toIntOrNull() }; if(p.size!=4) return h.endsWith(".local")
        return p[0]==10 || (p[0]==192 && p[1]==168) || (p[0]==172 && p[1] in 16..31) || (p[0]==169 && p[1]==254)
    }
}

private class PromptAssets(private val context: Context) {
    val schema: JSONObject get()=JSONObject(context.assets.open("ai/proposal-schema.json").bufferedReader().use { it.readText() }).apply { remove("\$schema"); remove("\$id") }
    fun input(turn: ModelTurn): String=buildString {
        append(context.assets.open("ai/aic-ir-0.1.md").bufferedReader().use { it.readText() })
        val words=turn.prompt.lowercase()
        if(turn.operation==AiOperation.CREATE) {
            val reference=when {
                "counter" in words || "increment" in words || "decrement" in words -> "counter"
                "calculator" in words || "calculate" in words -> "calculator"
                "note" in words || "history" in words || "persist" in words || "store" in words -> "notes"
                else -> "hello"
            }
            append("\n\nVALIDATED STARTING EXAMPLE (copy and modify the complete program; never replace source with a short label):\n")
                .append(context.assets.open("templates/$reference.aic").bufferedReader().use { it.readText() })
        }
        turn.currentSource?.let { current ->
            append("\n\nCURRENT PROGRAM (this is the only patch base; preserve its package and modify it):\n").append(current).append('\n')
            val rename=Regex("(?i)rename(?: the)? application title to\\s+['\"]?([^'\".\\n]+)").find(turn.prompt)?.groupValues?.get(1)?.trim()
            val oldName=Regex("""\bapp\s+"([^"]+)"""").find(current)?.groupValues?.get(1)
            if(rename!=null && oldName!=null) append("\nREQUIRED RENAME: change the declaration `app \"").append(oldName).append("\"` to `app \"").append(rename).append("\"`. The returned source must differ from CURRENT PROGRAM.\n")
            val persistenceIntent=words.contains("persist") || words.contains("between application launches") || words.contains("between launches")
            if(persistenceIntent && Regex("""\bstate\s+count:\s*i32""").containsMatchIn(current)) append("""

REQUIRED COUNTER PERSISTENCE TRANSFORMATION:
1. Inside the app block, before `activity MainActivity`, add exactly:
   capability persistence.key_value
   preference saved_count: i32 = 0
2. Keep `state count: i32 = 0`.
3. As the first statement in `on_create`, add:
   count = preference.get(saved_count)
4. In every counter click handler, immediately after changing `count`, add:
   preference.set(saved_count, count)
5. The capability and preference declarations must be inside the app braces. Never append anything after the final app brace.
""")
            val verticalCenterIntent=("center" in words || "centre" in words) && ("vertical" in words || "vertically" in words)
            if(verticalCenterIntent) append("""

REQUIRED VERTICAL CENTERING TRANSFORMATION USING SUPPORTED AIC IR:
Make only these four edits to CURRENT PROGRAM. Copy every other line unchanged. Do not add replacement or duplicate controls, colors, text updates, persistence calls, or layout calls.
1. In `on_create`, declare exactly two new empty spacer views immediately after the existing root declaration:
   let top_spacer = android.text_view(text: "")
   let bottom_spacer = android.text_view(text: "")
2. Give each spacer equal remaining space:
   android.set_layout(view: top_spacer, width: match_parent, height: wrap_content, weight: 1)
   android.set_layout(view: bottom_spacer, width: match_parent, height: wrap_content, weight: 1)
3. Add `top_spacer` to root immediately before the first existing visible element.
4. Add `bottom_spacer` immediately after the last existing visible element.
The final program must contain the same existing view declarations exactly once. It may contain only the two additional view declarations above. Use only weight 0 or 1. AIC IR has no gravity, alignment, padding, margin, or fluent `.set_*` calls. Do not append statements after the app's final brace.
""")
        }
        turn.diagnostics?.let { append("\nLOCAL VALIDATOR FEEDBACK AND REJECTED OUTPUT FROM THE PREVIOUS ATTEMPT:\n").append(it).append("\nRepair the complete AIC source.\n") }
        append("\nFINAL TASK\nOperation: ").append(turn.operation.wire).append("\nUser intent: ").append(turn.prompt)
            .append("\nThe `source` JSON field must contain the entire compilable AIC program, beginning with `aic_version 0.1`. For patch operations it must contain the requested actual source change; never return the original unchanged program. Do not summarize it or return a filename.\n")
    }
    val system="Translate Android app intent into the closed AIC IR 0.1 language. Output only the required JSON object. Never emit binaries, keys, signing data, commands, Java, or Kotlin."
}

private fun postJson(url: String, headers: Map<String,String>, body: JSONObject): JSONObject {
    val connection=(URL(url).openConnection() as HttpURLConnection).apply {
        requestMethod="POST"; connectTimeout=30000; readTimeout=120000; doOutput=true
        setRequestProperty("Content-Type","application/json"); headers.forEach { (k,v)->setRequestProperty(k,v) }
    }
    connection.outputStream.use { it.write(body.toString().toByteArray()) }
    val status=connection.responseCode
    val response=(if(status in 200..299) connection.inputStream else connection.errorStream)?.bufferedReader()?.use { it.readText() }.orEmpty()
    if(status !in 200..299) {
        val message=runCatching { JSONObject(response).optJSONObject("error")?.optString("message") ?: response }.getOrDefault(response).take(500)
        error("Provider request failed (HTTP $status): $message")
    }
    return JSONObject(response)
}

class OpenAiProvider(context: Context, private val apiKey: String, private val model: String): ModelProvider {
    private val assets=PromptAssets(context)
    override fun generate(turn: ModelTurn): ModelAnswer {
        require(apiKey.isNotBlank()) { "OpenAI API key is not configured" }
        val body=JSONObject().put("model",model).put("store",false).put("max_output_tokens",12000).put("instructions",assets.system).put("input",assets.input(turn))
            .put("text",JSONObject().put("format",JSONObject().put("type","json_schema").put("name","aic_model_proposal").put("strict",true).put("schema",assets.schema)))
        val json=postJson("https://api.openai.com/v1/responses",mapOf("Authorization" to "Bearer $apiKey"),body)
        check(json.optString("status")=="completed") { "OpenAI response was ${json.optString("status","incomplete")}" }
        val output=json.optJSONArray("output") ?: JSONArray(); var text: String?=null
        for(i in 0 until output.length()) { val content=output.getJSONObject(i).optJSONArray("content") ?: continue
            for(j in 0 until content.length()) if(content.getJSONObject(j).optString("type")=="output_text") text=content.getJSONObject(j).getString("text") }
        return ModelAnswer("openai",model,json.optString("id").ifBlank { null },text ?: error("OpenAI response contained no output text"))
    }
}

class ClaudeProvider(context: Context, private val apiKey: String, private val model: String): ModelProvider {
    private val assets=PromptAssets(context)
    override fun generate(turn: ModelTurn): ModelAnswer {
        require(apiKey.isNotBlank()) { "Claude API key is not configured" }
        val body=JSONObject().put("model",model).put("max_tokens",12000).put("system",assets.system)
            .put("messages",JSONArray().put(JSONObject().put("role","user").put("content",assets.input(turn))))
            .put("output_config",JSONObject().put("format",JSONObject().put("type","json_schema").put("schema",assets.schema)))
        val json=postJson("https://api.anthropic.com/v1/messages",mapOf("x-api-key" to apiKey,"anthropic-version" to "2023-06-01"),body)
        val content=json.optJSONArray("content") ?: JSONArray(); val text=(0 until content.length()).map { content.getJSONObject(it) }.firstOrNull { it.optString("type")=="text" }?.optString("text")
        return ModelAnswer("claude",model,json.optString("id").ifBlank { null },text ?: error("Claude response contained no text"))
    }
}

class ChatCompletionsProvider(context: Context, private val config: ProviderConfig, private val endpoint: String): ModelProvider {
    private val assets=PromptAssets(context)
    override fun generate(turn: ModelTurn): ModelAnswer {
        if(config.kind.credentialRequired) require(config.credential.isNotBlank()) { "${config.kind.label} credential is not configured" }
        val messages=JSONArray().put(JSONObject().put("role","system").put("content",assets.system)).put(JSONObject().put("role","user").put("content",assets.input(turn)))
        val format=JSONObject().put("type","json_schema").put("json_schema",JSONObject().put("name","aic_model_proposal").put("strict",true).put("schema",assets.schema))
        val body=JSONObject().put("model",config.model).put("messages",messages).put("stream",false).put("max_tokens",12000).put("response_format",format)
        if(config.kind==ProviderKind.OPENROUTER) body.put("provider",JSONObject().put("require_parameters",true))
        val headers=if(config.credential.isBlank()) emptyMap() else mapOf("Authorization" to "Bearer ${config.credential}")
        val json=postJson(endpoint,headers,body); val text=json.getJSONArray("choices").getJSONObject(0).getJSONObject("message").getString("content")
        return ModelAnswer(config.kind.id,config.model,json.optString("id").ifBlank { null },text)
    }
}

class OllamaProvider(context: Context, private val config: ProviderConfig): ModelProvider {
    private val assets=PromptAssets(context)
    override fun generate(turn: ModelTurn): ModelAnswer {
        val messages=JSONArray().put(JSONObject().put("role","system").put("content",assets.system)).put(JSONObject().put("role","user").put("content",assets.input(turn)))
        val body=JSONObject().put("model",config.model).put("messages",messages).put("stream",false).put("format",assets.schema)
            .put("options",JSONObject().put("temperature",0))
        val json=postJson(ModelProviderFactory.localEndpoint(config.baseUrl,"/api/chat"),emptyMap(),body)
        return ModelAnswer("ollama",config.model,null,json.getJSONObject("message").getString("content"))
    }
}
