package com.reloaded.android.ui.launcher

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.io.IOException
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

private data class SaveFileInfo(val name: String, val size: Long, val modified: Long)

@Composable
fun SaveManagerScreen(onNavigateBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var files by remember { mutableStateOf<List<SaveFileInfo>>(emptyList()) }
    var backupCount by remember { mutableIntStateOf(0) }
    var busy by remember { mutableStateOf(true) }
    var message by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    val dateFormat = remember { SimpleDateFormat("dd MMM yyyy · HH:mm", Locale.getDefault()) }

    suspend fun refresh() {
        val snapshot = withContext(Dispatchers.IO) {
            val saves = SaveArchive.files(GameLibrary.saveDirectory(context))
                .map { SaveFileInfo(it.name, it.length(), it.lastModified()) }
            val backups = GameLibrary.backupsDirectory(context).listFiles()
                ?.count { it.isFile && it.extension.equals("zip", ignoreCase = true) } ?: 0
            saves to backups
        }
        files = snapshot.first
        backupCount = snapshot.second
    }

    LaunchedEffect(Unit) {
        try { refresh() }
        catch (exception: Exception) { error = exception.message ?: "No se pudieron leer las partidas." }
        finally { busy = false }
    }
    BackHandler { if (!busy) onNavigateBack() }

    val exporter = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/zip")) { uri ->
        if (uri != null) scope.launch {
            busy = true
            message = null
            error = null
            try {
                withContext(Dispatchers.IO) {
                    val saves = SaveArchive.files(GameLibrary.saveDirectory(context))
                    val output = context.contentResolver.openOutputStream(uri, "wt")
                        ?: throw IOException("No se pudo abrir el destino.")
                    output.use { SaveArchive.write(saves, it) }
                }
                message = "Copia ZIP exportada. Incluye todos los archivos de tus partidas."
            } catch (exception: Exception) {
                error = exception.message ?: "No se pudo exportar la copia."
            } finally { busy = false }
        }
    }

    Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Column(
            Modifier.fillMaxSize().safeDrawingPadding().verticalScroll(rememberScrollState())
                .padding(horizontal = 24.dp, vertical = 20.dp),
            verticalArrangement = Arrangement.spacedBy(24.dp)
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                IconButton(onClick = onNavigateBack, enabled = !busy) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, "Volver a la biblioteca")
                }
                Text("Tu biblioteca", style = MaterialTheme.typography.titleMedium)
            }
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Partidas guardadas", style = MaterialTheme.typography.headlineLarge, fontWeight = FontWeight.Bold)
                Text("Pokémon Reloaded", style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text("Guarda dentro del juego antes de crear una copia. El ZIP conserva los archivos de progreso y sus datos asociados.",
                    style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            if (busy) LinearProgressIndicator(Modifier.fillMaxWidth())
            message?.let { Text(it, color = MaterialTheme.colorScheme.primary, style = MaterialTheme.typography.bodyMedium) }
            error?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium) }
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(18.dp),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
            ) {
                Column(Modifier.padding(22.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                    Text("Tus archivos", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                    if (files.isEmpty() && !busy) {
                        Text("Todavía no hay partidas guardadas. Cuando guardes tu progreso en el juego, aparecerá aquí.",
                            color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodyMedium)
                    }
                    files.forEachIndexed { index, file ->
                        if (index > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outline)
                        Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Text(file.name, style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.Medium)
                            Text("${formatSize(file.size)} · ${dateFormat.format(Date(file.modified))}",
                                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                    Button(
                        onClick = { exporter.launch("reloaded-partidas-${SimpleDateFormat("yyyyMMdd-HHmmss", Locale.ROOT).format(Date())}.zip") },
                        enabled = files.isNotEmpty() && !busy,
                        modifier = Modifier.fillMaxWidth().height(50.dp),
                        shape = RoundedCornerShape(12.dp)
                    ) { Text("Exportar copia ZIP") }
                }
            }
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Copias en este dispositivo", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                Text("$backupCount ${if (backupCount == 1) "copia local" else "copias locales"}. Exporta también un ZIP fuera de la app para conservarlo si la desinstalas.",
                    style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                OutlinedButton(
                    enabled = files.isNotEmpty() && !busy,
                    shape = RoundedCornerShape(12.dp),
                    onClick = {
                        scope.launch {
                            busy = true
                            message = null
                            error = null
                            try {
                                withContext(Dispatchers.IO) {
                                    val directory = GameLibrary.backupsDirectory(context).apply { mkdirs() }
                                    val target = File(directory, "partidas-${System.currentTimeMillis()}.zip")
                                    target.outputStream().use {
                                        SaveArchive.write(SaveArchive.files(GameLibrary.saveDirectory(context)), it)
                                    }
                                }
                                refresh()
                                message = "Copia local creada."
                            } catch (exception: Exception) {
                                error = exception.message ?: "No se pudo crear la copia."
                            } finally { busy = false }
                        }
                    }
                ) { Text("Crear copia local") }
            }
        }
    }
}

private fun formatSize(bytes: Long): String = when {
    bytes >= 1024 * 1024 -> String.format(Locale.getDefault(), "%.1f MB", bytes / (1024.0 * 1024))
    bytes >= 1024 -> String.format(Locale.getDefault(), "%.1f KB", bytes / 1024.0)
    else -> "$bytes B"
}
