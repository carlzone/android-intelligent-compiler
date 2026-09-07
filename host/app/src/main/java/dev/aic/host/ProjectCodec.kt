package dev.aic.host

import java.io.*
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.util.Properties
import java.util.zip.ZipEntry
import java.util.zip.ZipInputStream
import java.util.zip.ZipOutputStream

data class ProjectData(val name: String, val source: String, val profile: String = "android-35", val optLevel: Int = 1)

/** Canonical portable data only: no paths, IDs, build products or signing material. */
object ProjectCodec {
    const val MAX_SOURCE = 1024 * 1024
    const val MAX_ARCHIVE = 2 * 1024 * 1024
    fun validate(project: ProjectData): ProjectData {
        require(project.name.isNotBlank() && project.name.length <= 120 && project.name.none { it.isISOControl() }) { "Project name must contain 1–120 printable characters" }
        require(project.source.toByteArray(Charsets.UTF_8).size <= MAX_SOURCE) { "Source exceeds 1 MiB" }
        require(project.profile == "android-35") { "Unsupported API profile: ${project.profile}" }
        require(project.optLevel in 0..1) { "Optimization level must be 0 or 1" }
        return project
    }
    fun readLimited(input: InputStream, limit: Int): ByteArray {
        val out=ByteArrayOutputStream()
        val buffer=ByteArray(8192)
        while(true) {
            val n=input.read(buffer)
            if(n < 0) break
            require(out.size().toLong()+n <= limit) { "Input exceeds $limit bytes" }
            out.write(buffer,0,n)
        }
        return out.toByteArray()
    }
    fun utf8(bytes: ByteArray): String = Charsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT)
        .onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString()
    fun encode(project: ProjectData): ByteArray {
        validate(project)
        val meta=Properties().apply {
            setProperty("formatVersion","1"); setProperty("name",project.name)
            setProperty("profile",project.profile); setProperty("optLevel",project.optLevel.toString())
        }
        val metadata=StringWriter().also { meta.store(it,null) }.toString().toByteArray(Charsets.UTF_8)
        val out=ByteArrayOutputStream()
        ZipOutputStream(out).use { zip ->
            for ((name,bytes) in listOf("project.properties" to metadata,"source.aic" to project.source.toByteArray(Charsets.UTF_8))) {
                zip.putNextEntry(ZipEntry(name).apply { time=0 }); zip.write(bytes); zip.closeEntry()
            }
        }
        return out.toByteArray()
    }
    fun decode(bytes: ByteArray): ProjectData {
        require(bytes.size <= MAX_ARCHIVE) { "Project archive exceeds 2 MiB" }
        val entries=mutableMapOf<String,ByteArray>()
        ZipInputStream(ByteArrayInputStream(bytes)).use { zip ->
            while(true) {
                val entry=zip.nextEntry ?: break
                require(entry.name in setOf("project.properties","source.aic") && !entry.isDirectory) { "Unexpected archive entry: ${entry.name}" }
                require(!entries.containsKey(entry.name)) { "Duplicate archive entry: ${entry.name}" }
                entries[entry.name]=readLimited(zip,if(entry.name == "source.aic") MAX_SOURCE else 16384)
                zip.closeEntry()
            }
        }
        require(entries.keys == setOf("project.properties","source.aic")) { "Archive must contain project.properties and source.aic" }
        val props=Properties().apply { load(StringReader(utf8(entries.getValue("project.properties")))) }
        require(props.stringPropertyNames() == setOf("formatVersion","name","profile","optLevel")) { "Unsupported project metadata" }
        require(props.getProperty("formatVersion") == "1") { "Unsupported project format version" }
        return validate(ProjectData(props.getProperty("name"),utf8(entries.getValue("source.aic")),props.getProperty("profile"),props.getProperty("optLevel").toInt()))
    }
}
