package dev.aic.host

import org.json.JSONArray
import org.json.JSONObject
import java.security.MessageDigest

enum class AiOperation(val wire: String) { CREATE("create"), PATCH("patch") }

data class ModelTurn(
    val operation: AiOperation,
    val prompt: String,
    val currentSource: String?,
    val attempt: Int,
    val diagnostics: String?
)

data class ModelAnswer(val provider: String, val model: String, val responseId: String?, val rawProposal: String)
data class AiProposal(val operation: AiOperation, val source: String, val summary: String, val touchedAreas: List<String>)
data class AiOutcome(val proposal: AiProposal?, val diagnostic: String, val attempts: Int, val originalHash: String)

fun interface ModelProvider { fun generate(turn: ModelTurn): ModelAnswer }

object AiProtocol {
    const val SCHEMA_VERSION="aic.model-proposal/1"
    val AREAS=setOf("metadata","state","functions","capabilities","persistence","ui","events")

    fun parse(answer: ModelAnswer, expected: AiOperation): AiProposal {
        require(answer.rawProposal.toByteArray().size <= ProjectCodec.MAX_SOURCE + 4096) { "AIC7002 schema: Model response is too large" }
        val value=JSONObject(answer.rawProposal)
        val actual=value.keys().asSequence().toSet()
        val required=setOf("schema_version","operation","source","summary","touched_areas")
        require(actual==required) { "AIC7002 schema: Response fields must exactly match the proposal schema" }
        require(value.getString("schema_version")==SCHEMA_VERSION) { "AIC7002 schema: Unsupported proposal schema" }
        require(value.getString("operation")==expected.wire) { "AIC7002 schema: Operation does not match the request" }
        val source=value.getString("source")
        require(source.isNotBlank() && source.toByteArray().size <= ProjectCodec.MAX_SOURCE) { "AIC7002 schema: Invalid source size" }
        require(source.trimStart().startsWith("aic_version 0.1")) { "AIC7002 schema: source must be a complete AIC program beginning with `aic_version 0.1`" }
        val summary=value.getString("summary")
        require(summary.isNotBlank() && summary.length<=500) { "AIC7002 schema: Invalid summary" }
        val array=value.getJSONArray("touched_areas")
        require(array.length() in 1..8) { "AIC7002 schema: Invalid touched areas" }
        val areas=(0 until array.length()).map { array.getString(it) }
        require(areas.distinct().size==areas.size && areas.all { it in AREAS }) { "AIC7002 schema: Unknown or duplicate touched area" }
        return AiProposal(expected,source,summary,areas)
    }

    fun packageName(source: String): String? = Regex("""\bpackage\s+"([a-zA-Z0-9_.]+)"""").find(source)?.groupValues?.get(1)
    fun hash(text: String): String=MessageDigest.getInstance("SHA-256").digest(text.toByteArray()).joinToString("") { "%02x".format(it) }

    fun knownEdit(prompt: String, source: String): String? {
        val words=prompt.lowercase()
        var edited=source
        var recognized=false
        val rename=Regex("(?i)rename(?: the)? application title to\\s+['\"]?([^'\".\\n]+)").find(prompt)?.groupValues?.get(1)?.trim()
        if(!rename.isNullOrBlank()) {
            val declaration=Regex("\\bapp\\s+\"([^\"]+)\"").find(edited) ?: return null
            val nameRange=declaration.groups[1]?.range ?: return null
            edited=edited.replaceRange(nameRange,rename)
            recognized=true
        }
        val vertical=("center" in words || "centre" in words) && ("vertical" in words || "vertically" in words)
        if(!vertical) return if(recognized) edited else null
        if("let top_spacer" in edited || "let bottom_spacer" in edited) return if(recognized) edited else null
        val root=Regex("(?m)^([ \\t]*)let root = android\\.linear_layout\\(orientation: vertical\\)[ \\t]*$").find(edited) ?: return if(recognized) edited else null
        val indent=root.groupValues[1]
        edited=edited.replaceRange(root.range.last+1,root.range.last+1,
            "\n${indent}let top_spacer = android.text_view(text: \"\")\n${indent}let bottom_spacer = android.text_view(text: \"\")")
        val firstChild=Regex("(?m)^([ \\t]*)android\\.add_view\\(parent: root, child: [A-Za-z_][A-Za-z0-9_]*\\)[ \\t]*$").find(edited) ?: return null
        edited=edited.replaceRange(firstChild.range.first,firstChild.range.first,
            "${firstChild.groupValues[1]}android.set_layout(view: top_spacer, width: match_parent, height: wrap_content, weight: 1)\n"+
                "${firstChild.groupValues[1]}android.set_layout(view: bottom_spacer, width: match_parent, height: wrap_content, weight: 1)\n"+
                "${firstChild.groupValues[1]}android.add_view(parent: root, child: top_spacer)\n")
        val content=Regex("(?m)^([ \\t]*)android\\.set_content_view\\(root\\)[ \\t]*$").find(edited) ?: return null
        return edited.replaceRange(content.range.first,content.range.first,
            "${content.groupValues[1]}android.add_view(parent: root, child: bottom_spacer)\n")
    }

    fun review(before: String, after: String): String {
        val a=before.lines(); val b=after.lines(); val out=StringBuilder()
        val max=maxOf(a.size,b.size)
        for(i in 0 until max) {
            val old=a.getOrNull(i); val new=b.getOrNull(i)
            if(old!=new) {
                old?.let { out.append("- ").append(it).append('\n') }
                new?.let { out.append("+ ").append(it).append('\n') }
            }
            if(out.length>12000) return out.append("…\n").toString()
        }
        return out.toString().ifBlank { "No textual changes." }
    }

    fun diagnostics(json: JSONObject): String {
        val a=json.optJSONArray("diagnostics") ?: JSONArray()
        return (0 until a.length()).joinToString("\n") { i ->
            val d=a.getJSONObject(i); val location=d.optJSONObject("location")
            buildString {
                append(d.optString("stage","validate")).append(' ').append(d.optString("code","AIC7099")).append(": ").append(d.optString("message","Validation failed"))
                if(location!=null) append(" at ").append(location.optInt("line")).append(':').append(location.optInt("column"))
            }
        }.ifBlank { "validate AIC7099: Validation failed" }
    }
}

class AiRepairLoop(private val provider: ModelProvider, private val validate: (String)->JSONObject, private val record: (ModelTurn,ModelAnswer,String)->Unit = {_,_,_->}) {
    fun run(operation: AiOperation, prompt: String, current: String?): AiOutcome {
        require(prompt.isNotBlank() && prompt.length<=4000) { "AIC7001 request: Prompt must contain 1–4000 characters" }
        if(operation==AiOperation.PATCH) require(!current.isNullOrBlank()) { "AIC7001 request: Patch requires a current project" }
        val original=current.orEmpty(); val knownEdit=if(operation==AiOperation.PATCH) AiProtocol.knownEdit(prompt,original) else null; var feedback: String?=null
        repeat(3) { index ->
            val turn=ModelTurn(operation,prompt,current,index+1,feedback)
            val answer=try { provider.generate(turn) } catch(e: Exception) {
                feedback="provider AIC7003: ${e.message ?: e.javaClass.simpleName}"; return@repeat
            }
            var proposal=try { AiProtocol.parse(answer,operation) } catch(e: Exception) {
                feedback=(e.message ?: "AIC7002 schema: Invalid response")+"\nRejected response:\n"+answer.rawProposal.take(4000)
                record(turn,answer,feedback!!); return@repeat
            }
            if(knownEdit!=null) {
                val renamed=Regex("(?i)rename(?: the)? application title to").containsMatchIn(prompt)
                proposal=proposal.copy(source=knownEdit,summary=if(renamed) "Renamed application title" else "Centered existing elements vertically",
                    touchedAreas=listOf(if(renamed) "metadata" else "ui"))
            }
            if(operation==AiOperation.PATCH && AiProtocol.packageName(proposal.source)!=AiProtocol.packageName(original)) {
                feedback="policy AIC7004: Patch must preserve the application package"; record(turn,answer,feedback!!); return@repeat
            }
            if(operation==AiOperation.PATCH && proposal.source==original) {
                feedback="policy AIC7006: The patch made no source changes despite claiming an action. Modify the requested AIC elements and return the complete changed program."
                record(turn,answer,feedback!!); return@repeat
            }
            val result=validate(proposal.source)
            if(result.optBoolean("ok")) { record(turn,answer,"accepted"); return AiOutcome(proposal,"Proposal validated. Review before applying.",index+1,AiProtocol.hash(original)) }
            feedback=AiProtocol.diagnostics(result)+"\nRejected AIC source:\n"+proposal.source.take(12000)
            record(turn,answer,feedback!!)
        }
        return AiOutcome(null,"AIC7005 repair: Validation failed after 3 attempts. Last valid source was preserved.\n${feedback.orEmpty()}",3,AiProtocol.hash(original))
    }
}
