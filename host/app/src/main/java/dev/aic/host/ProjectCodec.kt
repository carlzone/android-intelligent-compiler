package dev.aic.host

import java.io.*
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.util.Properties
import java.security.MessageDigest
import java.util.zip.ZipEntry
import java.util.zip.ZipInputStream
import java.util.zip.ZipOutputStream

data class ProjectData(val name: String, val source: String, val profile: String = "android-35", val optLevel: Int = 1,
    val images: Map<String,ByteArray> = emptyMap()) {
    override fun equals(other: Any?): Boolean = other is ProjectData && name==other.name && source==other.source &&
        profile==other.profile && optLevel==other.optLevel && images.keys==other.images.keys && images.all { (name,bytes) -> bytes.contentEquals(other.images[name]) }
    override fun hashCode(): Int = listOf(name,source,profile,optLevel,images.toSortedMap().mapValues { it.value.contentHashCode() }).hashCode()
}

/** Canonical portable data only: no paths, IDs, build products or signing material. */
object ProjectCodec {
    const val MAX_SOURCE = 1024 * 1024
    const val MAX_IMAGE = 4 * 1024 * 1024
    const val MAX_ARCHIVE = 10 * 1024 * 1024
    fun validate(project: ProjectData): ProjectData {
        require(project.name.isNotBlank() && project.name.length <= 120 && project.name.none { it.isISOControl() }) { "Project name must contain 1–120 printable characters" }
        require(project.source.toByteArray(Charsets.UTF_8).size <= MAX_SOURCE) { "Source exceeds 1 MiB" }
        require(project.profile == "android-35") { "Unsupported API profile: ${project.profile}" }
        require(project.optLevel in 0..1) { "Optimization level must be 0 or 1" }
        require(project.images.size <= 64) { "A project may contain at most 64 images" }
        for((name,bytes) in project.images) {
            require(Regex("[a-z][a-z0-9_]{0,62}\\.(png|webp)").matches(name)) { "Invalid image asset name: $name" }
            require(bytes.isNotEmpty() && bytes.size <= MAX_IMAGE) { "Image asset $name exceeds the supported size" }
            val png=bytes.size>=24 && bytes.copyOfRange(0,8).contentEquals(byteArrayOf(0x89.toByte(),0x50,0x4e,0x47,0x0d,0x0a,0x1a,0x0a)) &&
                bytes.copyOfRange(12,16).contentEquals("IHDR".toByteArray()) && dimension(bytes,16,true) in 1..8192 && dimension(bytes,20,true) in 1..8192
            val webp=bytes.size>=16 && bytes.copyOfRange(0,4).contentEquals("RIFF".toByteArray()) && bytes.copyOfRange(8,12).contentEquals("WEBP".toByteArray()) &&
                dimension(bytes,4,false).toLong()+8==bytes.size.toLong() && String(bytes,12,4,Charsets.US_ASCII) in setOf("VP8 ","VP8L","VP8X")
            require((name.endsWith(".png") && png) || (name.endsWith(".webp") && webp)) { "Image asset $name has invalid content" }
        }
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
            setProperty("formatVersion","2"); setProperty("name",project.name)
            setProperty("profile",project.profile); setProperty("optLevel",project.optLevel.toString())
            setProperty("assetDigest",assetDigest(project.images))
        }
        val metadata=StringWriter().also { meta.store(it,null) }.toString().toByteArray(Charsets.UTF_8)
        val out=ByteArrayOutputStream()
        ZipOutputStream(out).use { zip ->
            val entries=listOf("project.properties" to metadata,"source.aic" to project.source.toByteArray(Charsets.UTF_8))+
                project.images.toSortedMap().map { (name,bytes) -> "assets/images/$name" to bytes }
            for ((name,bytes) in entries) {
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
                val image=Regex("assets/images/([a-z][a-z0-9_]{0,62}\\.(?:png|webp))").matchEntire(entry.name)
                require((entry.name in setOf("project.properties","source.aic") || image!=null) && !entry.isDirectory) { "Unexpected archive entry: ${entry.name}" }
                require(!entries.containsKey(entry.name)) { "Duplicate archive entry: ${entry.name}" }
                entries[entry.name]=readLimited(zip,when { entry.name=="source.aic" -> MAX_SOURCE; image!=null -> MAX_IMAGE; else -> 16384 })
                zip.closeEntry()
            }
        }
        require(entries.keys.containsAll(setOf("project.properties","source.aic"))) { "Archive must contain project.properties and source.aic" }
        val props=Properties().apply { load(StringReader(utf8(entries.getValue("project.properties")))) }
        val version=props.getProperty("formatVersion")
        val common=setOf("formatVersion","name","profile","optLevel")
        require((version=="1" && props.stringPropertyNames()==common) || (version=="2" && props.stringPropertyNames()==common+"assetDigest")) { "Unsupported project metadata or format version" }
        val images=entries.filterKeys { it.startsWith("assets/images/") }.mapKeys { it.key.removePrefix("assets/images/") }
        if(version=="1") require(images.isEmpty()) { "Format 1 projects cannot contain assets" }
        if(version=="2") require(props.getProperty("assetDigest")==assetDigest(images)) { "Project asset checksum mismatch" }
        return validate(ProjectData(props.getProperty("name"),utf8(entries.getValue("source.aic")),props.getProperty("profile"),props.getProperty("optLevel").toInt(),images))
    }
    private fun assetDigest(images: Map<String,ByteArray>): String {
        val digest=MessageDigest.getInstance("SHA-256")
        for((name,bytes) in images.toSortedMap()) { digest.update(name.toByteArray(Charsets.UTF_8)); digest.update(0); digest.update(bytes) }
        return digest.digest().joinToString("") { "%02x".format(it) }
    }
    private fun dimension(bytes: ByteArray, at: Int, bigEndian: Boolean): Int {
        var value=0
        for(index in 0..3) { val offset=if(bigEndian) index else 3-index; value=(value shl 8) or (bytes[at+offset].toInt() and 0xff) }
        return value
    }
}
