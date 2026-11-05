package com.reloaded.android.ui.launcher

import android.content.Context
import java.io.File
import java.io.OutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

/** The alpha has one installed game; future library profiles can own separate roots. */
object GameLibrary {
    fun executable(context: Context): File? {
        val roots = listOf(
            File(context.filesDir, "game_extracted"),
            File(context.filesDir, "reloaded_data/game_extracted")
        )
        return roots.asSequence().filter { it.isDirectory }.flatMap { root ->
            root.walkTopDown().maxDepth(3).onEnter {
                it == root || it.name.lowercase() !in setOf("data", "backups")
            }.asSequence()
        }.filter { it.isFile && it.extension.equals("exe", ignoreCase = true) }
            .maxByOrNull { it.length() }
    }

    fun saveDirectory(context: Context): File =
        File(executable(context)?.parentFile ?: File(context.filesDir, "game_extracted"), "data/partidas")

    fun backupsDirectory(context: Context): File = File(context.filesDir, "save_backups")
}

object SaveArchive {
    fun files(directory: File): List<File> = directory.listFiles()
        ?.filter { it.isFile && !it.name.startsWith(".") }
        ?.sortedBy { it.name.lowercase() }
        .orEmpty()

    fun write(files: List<File>, destination: OutputStream) {
        require(files.isNotEmpty()) { "Todavía no hay partidas para exportar." }
        ZipOutputStream(destination.buffered()).use { zip ->
            for (file in files) {
                require(file.isFile) { "No se encontró ${file.name}." }
                zip.putNextEntry(ZipEntry("data/partidas/${file.name}").apply { time = file.lastModified() })
                file.inputStream().use { it.copyTo(zip) }
                zip.closeEntry()
            }
        }
    }
}
