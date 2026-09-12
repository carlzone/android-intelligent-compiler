package dev.aic.host

import org.junit.Assert.*
import org.junit.Test
import java.io.ByteArrayOutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

class ProjectCodecTest {
    @Test fun roundTripPreservesAllCanonicalData() {
        val p=ProjectData("筆記 📝", "// 中文 😀\napp \"x\"\n",optLevel=0)
        assertEquals(p,ProjectCodec.decode(ProjectCodec.encode(p)))
    }
    @Test fun rejectsUntrustedArchivesAndEncoding() {
        fun zip(name: String,bytes: ByteArray): ByteArray {
            val out=ByteArrayOutputStream()
            ZipOutputStream(out).use { it.putNextEntry(ZipEntry(name)); it.write(bytes); it.closeEntry() }
            return out.toByteArray()
        }
        for(name in listOf("../source.aic","/source.aic","signing.key","x/source.aic"))
            assertThrows(IllegalArgumentException::class.java) { ProjectCodec.decode(zip(name,byteArrayOf())) }
        assertThrows(IllegalArgumentException::class.java) { ProjectCodec.decode(zip("source.aic",ByteArray(ProjectCodec.MAX_SOURCE+1))) }
        assertThrows(IllegalArgumentException::class.java) { ProjectCodec.decode(byteArrayOf(1,2,3)) }
        assertThrows(java.nio.charset.CharacterCodingException::class.java) { ProjectCodec.utf8(byteArrayOf(0xc3.toByte())) }
    }
    @Test fun rejectsUnsupportedSettings() {
        for(p in listOf(ProjectData("", ""),ProjectData("x","", "android-99"),ProjectData("x","",optLevel=2),ProjectData("x","a".repeat(ProjectCodec.MAX_SOURCE+1))))
            assertThrows(IllegalArgumentException::class.java) { ProjectCodec.encode(p) }
    }
    @Test fun formatTwoPreservesValidatedImages() {
        val png=byteArrayOf(0x89.toByte(),0x50,0x4e,0x47,0x0d,0x0a,0x1a,0x0a,0,0,0,13,0x49,0x48,0x44,0x52,0,0,0,1,0,0,0,1)
        val project=ProjectData("images","aic_version 0.2",images=mapOf("logo.png" to png))
        val decoded=ProjectCodec.decode(ProjectCodec.encode(project))
        assertEquals(project,decoded)
        assertArrayEquals(png,decoded.images.getValue("logo.png"))
        assertThrows(IllegalArgumentException::class.java) { ProjectCodec.encode(project.copy(images=mapOf("../logo.png" to png))) }
        assertThrows(IllegalArgumentException::class.java) { ProjectCodec.encode(project.copy(images=mapOf("logo.png" to byteArrayOf(1,2,3)))) }
    }
}
