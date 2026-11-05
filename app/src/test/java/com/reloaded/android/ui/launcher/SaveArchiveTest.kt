package com.reloaded.android.ui.launcher

import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.ByteArrayOutputStream
import java.util.zip.ZipInputStream

class SaveArchiveTest {
    @get:Rule val temporary = TemporaryFolder()

    @Test fun exportsAllSaveComponentsWithoutChangingTheirBytes() {
        val directory = temporary.newFolder("partidas")
        val expected = mapOf(
            "reloaded1.savc" to byteArrayOf(0, 7, -1, 0, 12),
            "reloaded1.data" to "datos asociados".toByteArray(),
            "reloaded1.nfo" to "nombre del entrenador".toByteArray(),
            "reloaded1_bk.savc" to byteArrayOf(3, 2, 1)
        )
        expected.forEach { (name, bytes) -> directory.resolve(name).writeBytes(bytes) }
        directory.resolve(".internal").writeText("excluded")
        directory.resolve("nested").mkdir()
        val output = ByteArrayOutputStream()
        SaveArchive.write(SaveArchive.files(directory), output)
        val actual = mutableMapOf<String, ByteArray>()
        ZipInputStream(output.toByteArray().inputStream()).use { zip ->
            while (true) {
                val entry = zip.nextEntry ?: break
                actual[entry.name] = zip.readBytes()
            }
        }
        assertEquals(expected.keys.map { "data/partidas/$it" }.toSet(), actual.keys)
        expected.forEach { (name, bytes) -> assertArrayEquals(bytes, actual["data/partidas/$name"]) }
    }

    @Test fun refusesEmptyExports() {
        val output = ByteArrayOutputStream()
        assertThrows(IllegalArgumentException::class.java) { SaveArchive.write(emptyList(), output) }
        assertEquals(0, output.size())
    }
}
