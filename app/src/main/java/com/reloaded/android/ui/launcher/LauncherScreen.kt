package com.reloaded.android.ui.launcher

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.reloaded.android.BuildConfig
import com.reloaded.android.bridge.NativeBridge
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.io.IOException

@Composable
fun LauncherScreen(
    onStartGame: () -> Unit,
    onNavigateToSaveManager: () -> Unit,
    isDarkMode: Boolean,
    onToggleDarkMode: (Boolean) -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var ready by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(true) }
    var gameTitle by remember { mutableStateOf("Pokémon Reloaded") }
    var status by remember { mutableStateOf("Buscando tu instalación…") }
    var error by remember { mutableStateOf<String?>(null) }
    var showAbout by remember { mutableStateOf(false) }

    suspend fun prepare(packageFile: File) {
        check(NativeBridge.isLoaded()) { "No se pudo iniciar el motor de esta versión." }
        val response = withContext(Dispatchers.IO) {
            JSONObject(NativeBridge.importPackageSafe(packageFile.absolutePath, context.cacheDir.absolutePath))
        }
        check(response.optString("status") == "ok") {
            response.optString("message", "No se pudo preparar este paquete.")
        }
        check(withContext(Dispatchers.IO) { GameLibrary.executable(context) != null }) {
            "El paquete no contiene un ejecutable compatible."
        }
        val title = response.optJSONObject("metadata")?.optString("title").orEmpty()
        gameTitle = if (title.contains("reloaded", ignoreCase = true) || title.isBlank()) {
            "Pokémon Reloaded"
        } else title
        ready = true
        status = "Instalado en este dispositivo"
    }

    LaunchedEffect(Unit) {
        NativeBridge.initSafe(context.filesDir.absolutePath)
        try {
            val existing = withContext(Dispatchers.IO) {
                GameLibrary.executable(context)
                    ?: File(context.cacheDir, "pokemon_reloaded_import.zip").takeIf { it.isFile && it.length() > 0 }
            }
            if (existing != null) prepare(existing)
            else status = "Importa tu copia del juego para empezar."
        } catch (exception: Exception) {
            ready = false
            error = exception.message ?: "No se pudo abrir la instalación."
        } finally {
            busy = false
        }
    }

    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) scope.launch {
            busy = true
            ready = false
            error = null
            status = "Copiando el paquete…"
            try {
                val packageFile = File(context.cacheDir, "game_import.zip")
                withContext(Dispatchers.IO) {
                    val input = context.contentResolver.openInputStream(uri)
                        ?: throw IOException("No se pudo leer el archivo seleccionado.")
                    input.use { source -> packageFile.outputStream().use { source.copyTo(it, 256 * 1024) } }
                    check(packageFile.length() > 0) { "El archivo seleccionado está vacío." }
                    val files = SaveArchive.files(GameLibrary.saveDirectory(context))
                    if (files.isNotEmpty()) {
                        val directory = GameLibrary.backupsDirectory(context).apply { mkdirs() }
                        val backup = File(directory, "antes-de-importar-${System.currentTimeMillis()}.zip")
                        backup.outputStream().use { SaveArchive.write(files, it) }
                    }
                }
                status = "Preparando el juego…"
                prepare(packageFile)
            } catch (exception: Exception) {
                error = exception.message ?: "No se pudo importar el juego."
                status = "Revisa el paquete y vuelve a intentarlo."
            } finally {
                busy = false
            }
        }
    }

    Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Column(
            Modifier.fillMaxSize().safeDrawingPadding().verticalScroll(rememberScrollState())
                .padding(horizontal = 24.dp, vertical = 28.dp),
            verticalArrangement = Arrangement.spacedBy(24.dp)
        ) {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text("Reloaded Engine", style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.onBackground)
                Spacer(Modifier.weight(1f))
                Text(BuildConfig.VERSION_NAME, style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Tu biblioteca", style = MaterialTheme.typography.headlineLarge,
                    fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.onBackground)
                Text("Clásicos de GameMaker, en Android.", style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
            }

            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
                shape = RoundedCornerShape(18.dp)
            ) {
                Column(Modifier.padding(22.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                    Text(if (ready) "EN TU BIBLIOTECA" else "PRIMER JUEGO COMPATIBLE",
                        style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
                    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(gameTitle, style = MaterialTheme.typography.headlineSmall,
                            fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.onSurface)
                        Text(status, style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    if (busy) {
                        LinearProgressIndicator(Modifier.fillMaxWidth(), color = MaterialTheme.colorScheme.primary)
                    }
                    error?.let {
                        Text(it, style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.error)
                    }
                    Button(
                        onClick = { if (ready) onStartGame() else picker.launch(arrayOf("application/zip", "application/x-zip-compressed")) },
                        enabled = !busy,
                        modifier = Modifier.fillMaxWidth().height(50.dp),
                        shape = RoundedCornerShape(12.dp)
                    ) {
                        Text(if (ready) "Abrir juego" else "Importar paquete ZIP", fontWeight = FontWeight.SemiBold)
                    }
                    if (ready && !busy) {
                        TextButton(onClick = { picker.launch(arrayOf("application/zip", "application/x-zip-compressed")) },
                            contentPadding = PaddingValues(0.dp)) {
                            Text("Importar otro paquete")
                        }
                    } else if (!busy) {
                        Text("Selecciona el ZIP completo del juego, con su ejecutable y su carpeta de datos.",
                            style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }

            OutlinedCard(
                onClick = onNavigateToSaveManager,
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(16.dp),
                enabled = !busy
            ) {
                Row(Modifier.fillMaxWidth().padding(20.dp), verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(5.dp)) {
                        Text("Partidas guardadas", style = MaterialTheme.typography.titleMedium,
                            color = MaterialTheme.colorScheme.onSurface)
                        Text("Consulta tus archivos y exporta una copia.", style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Icon(Icons.AutoMirrored.Filled.ArrowForward, "Abrir partidas", tint = MaterialTheme.colorScheme.primary)
                }
            }
            HorizontalDivider(color = MaterialTheme.colorScheme.outline)
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(3.dp)) {
                    Text("Apariencia", style = MaterialTheme.typography.titleSmall,
                        color = MaterialTheme.colorScheme.onSurface)
                    Text(if (isDarkMode) "Modo oscuro" else "Modo claro", style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                TextButton(onClick = { onToggleDarkMode(!isDarkMode) }) { Text("Cambiar") }
            }
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("Una biblioteca en crecimiento.", style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSurface)
                Text("Pokémon Reloaded es el primer juego de esta alpha. La compatibilidad con otros títulos se ampliará con pruebas y perfiles propios.",
                    style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                TextButton(onClick = { showAbout = true }, contentPadding = PaddingValues(0.dp)) {
                    Text("Acerca de Reloaded Engine")
                }
            }
        }
    }
    if (showAbout) {
        AlertDialog(
            onDismissRequest = { showAbout = false },
            title = { Text("Reloaded Engine") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text("${BuildConfig.VERSION_NAME} · Versión experimental para Android")
                    Text("Un launcher para conservar y disfrutar juegos de GameMaker. Esta primera alpha se centra en Pokémon Reloaded.")
                    Text("Basado en OpenGMK. Gracias a Klisee por el concepto y los parches de compatibilidad utilizados como referencia, y a F3R Productions por Pokémon Reloaded.")
                    Text("Software libre bajo GPL-2.0. Los juegos y sus recursos pertenecen a sus respectivos autores.")
                }
            },
            confirmButton = { TextButton(onClick = { showAbout = false }) { Text("Cerrar") } }
        )
    }
}
