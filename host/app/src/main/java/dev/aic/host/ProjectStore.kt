package dev.aic.host

import android.content.Context
import android.util.AtomicFile
import java.io.File
import java.util.UUID

data class StoredProject(val id: String, val data: ProjectData)
class ProjectStore(context: Context) {
    private val root=File(context.filesDir,"projects").apply { check(mkdirs() || isDirectory) { "Cannot open project storage" } }
    private fun file(id: String): File {
        require(runCatching { UUID.fromString(id).toString() == id }.getOrDefault(false)) { "Invalid project ID" }
        return File(root,"$id.aicproject")
    }
    fun list(): List<StoredProject> = root.listFiles().orEmpty().filter { it.extension == "aicproject" }
        .map { load(it.nameWithoutExtension) }.sortedBy { it.data.name.lowercase() }
    fun load(id: String): StoredProject = StoredProject(id,ProjectCodec.decode(AtomicFile(file(id)).openRead().use { ProjectCodec.readLimited(it,ProjectCodec.MAX_ARCHIVE) }))
    fun create(data: ProjectData): StoredProject = save(StoredProject(UUID.randomUUID().toString(),data))
    fun save(project: StoredProject): StoredProject {
        val bytes=ProjectCodec.encode(project.data)
        val atomic=AtomicFile(file(project.id))
        val stream=atomic.startWrite()
        try { stream.write(bytes); stream.fd.sync(); atomic.finishWrite(stream) } catch(e: Exception) { atomic.failWrite(stream); throw e }
        check(load(project.id) == project) { "Project save could not be committed; check available storage" }
        return project
    }
}
