package dev.aic.host

import android.content.Context
import org.json.JSONObject
import java.io.File
import java.time.Instant

class AiProvenance(context: Context) {
    private val file=File(context.filesDir,"ai/provenance.jsonl").apply { parentFile?.mkdirs() }
    @Synchronized fun record(projectId: String?, turn: ModelTurn, answer: ModelAnswer, result: String) {
        val row=JSONObject().put("version",1).put("time",Instant.now().toString()).put("project_id",projectId)
            .put("provider",answer.provider).put("model",answer.model).put("response_id",answer.responseId)
            .put("schema_version",AiProtocol.SCHEMA_VERSION).put("operation",turn.operation.wire).put("attempt",turn.attempt)
            .put("request",turn.prompt).put("base_ir_sha256",turn.currentSource?.let(AiProtocol::hash))
            .put("response",answer.rawProposal).put("result",result)
        file.appendText(row.toString()+"\n")
        if(file.length()>2L*1024*1024) file.writeText(file.readLines().takeLast(1000).joinToString("\n",postfix="\n"))
    }

    data class HistoryEntry(val time: String, val provider: String, val model: String, val operation: String, val attempt: Int, val prompt: String, val summary: String, val result: String)

    @Synchronized fun recent(limit: Int=25): List<HistoryEntry> {
        if(!file.isFile) return emptyList()
        return file.readLines().takeLast(limit.coerceIn(1,100)).mapNotNull { line -> runCatching {
            val row=JSONObject(line); val response=runCatching { JSONObject(row.optString("response")) }.getOrNull()
            HistoryEntry(row.optString("time"),row.optString("provider"),row.optString("model"),row.optString("operation"),row.optInt("attempt"),
                row.optString("request"),response?.optString("summary").orEmpty(),row.optString("result"))
        }.getOrNull() }.reversed()
    }

    @Synchronized fun clear() { if(file.exists() && !file.delete()) error("Could not clear AI history") }
}
