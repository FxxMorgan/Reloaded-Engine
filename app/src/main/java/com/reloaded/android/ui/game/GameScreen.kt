package com.reloaded.android.ui.game

import android.content.Context
import android.hardware.input.InputManager
import android.view.InputDevice
import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ChevronLeft
import androidx.compose.material.icons.filled.ChevronRight
import androidx.compose.material.icons.filled.DashboardCustomize
import androidx.compose.material.icons.filled.Keyboard
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import com.reloaded.android.bridge.AudioEnginePlayer
import com.reloaded.android.bridge.NativeBridge
import com.reloaded.android.bridge.NativeBridge.InputBits
import com.reloaded.android.ui.theme.BorderCrisp
import com.reloaded.android.ui.theme.SteelBlue
import com.reloaded.android.ui.theme.SurfacePureWhite
import com.reloaded.android.ui.theme.TextDeepSlate
import com.reloaded.android.ui.theme.TextMutedSlate
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import java.io.File
import java.util.concurrent.atomic.AtomicReference

data class DebugMetricsSnapshot(
    val stepsPerSec: Int = 0,
    val targetFps: Int = 30,
    val engineFps: Int = 30,
    val roomSpeed: Int = 30,
    val roomId: Int = 0,
    val roomName: String = "",
    val stepTimeMs: Float = 0f,
    val sleepTimeMs: Float = 0f,
    val instanceCount: Int = 0,
    val totalSteps: Long = 0L,
    val speedMultiplier: Int = 1
)

class StrictStepPacer {
    private var lastStartNs = System.nanoTime()

    fun reset() {
        lastStartNs = System.nanoTime()
    }

    @Throws(InterruptedException::class)
    fun awaitNext(targetHz: Int): Long {
        val safeHz = targetHz.coerceIn(15, 120)
        val periodNs = (1_000_000_000L + safeHz - 1) / safeHz
        val deadlineNs = lastStartNs + periodNs

        while (true) {
            if (Thread.currentThread().isInterrupted) {
                throw InterruptedException()
            }

            val remainingNs = deadlineNs - System.nanoTime()
            if (remainingNs <= 0L) break

            val sleepMs = remainingNs / 1_000_000L
            val sleepNs = (remainingNs % 1_000_000L).toInt()
            Thread.sleep(sleepMs, sleepNs)
        }

        val now = System.nanoTime()
        val elapsed = now - lastStartNs
        lastStartNs = now
        return elapsed
    }
}

@Composable
fun GameScreen(
    onExitToLauncher: () -> Unit
) {
    val context = LocalContext.current
    val audioPlayer = remember { AudioEnginePlayer() }
    val renderDispatcher = remember {
        java.util.concurrent.Executors.newSingleThreadExecutor { runnable ->
            Thread(runnable, "ReloadedRenderThread").apply { priority = Thread.MAX_PRIORITY }
        }.asCoroutineDispatcher()
    }
    var isRunning by remember { mutableStateOf(true) }
    var isPaused by remember { mutableStateOf(false) }
    var isSurfaceReady by remember { mutableStateOf(false) }
    var hasPhysicalController by remember { mutableStateOf(false) }

    // Live Metrics & Telemetry
    var speedMultiplier by remember { mutableIntStateOf(1) }
    val liveMetricsRef = remember { AtomicReference(DebugMetricsSnapshot()) }
    var isDebugOverlayVisible by remember { mutableStateOf(false) }
    var isDebugOverlayExpanded by remember { mutableStateOf(false) }
    var debugSnapshot by remember { mutableStateOf(DebugMetricsSnapshot()) }

    // UI state
    var isHudCollapsed by remember { mutableStateOf(true) }
    var isMacroBarVisible by remember { mutableStateOf(false) }
    var isLayoutEditing by remember { mutableStateOf(false) }
    var overlayOpacity by remember { mutableFloatStateOf(0.72f) }
    var showPauseDialog by remember { mutableStateOf(false) }

    // Audio playback lifecycle
    LaunchedEffect(Unit) {
        audioPlayer.start()
    }

    // Diagnostic Telemetry Poller (5 Hz - non-blocking for render thread)
    LaunchedEffect(isDebugOverlayVisible) {
        if (!isDebugOverlayVisible) return@LaunchedEffect
        while (isActive) {
            val base = liveMetricsRef.get()
            val json = NativeBridge.getEngineMetricsJsonSafe()
            var merged = base
            try {
                val obj = org.json.JSONObject(json)
                merged = base.copy(
                    engineFps = obj.optInt("current_fps", obj.optDouble("fps", base.engineFps.toDouble()).toInt()),
                    roomSpeed = obj.optInt("room_speed", base.roomSpeed),
                    roomId = obj.optInt("room_id", base.roomId),
                    roomName = obj.optString("room_name", base.roomName),
                    instanceCount = obj.optInt("instance_count", base.instanceCount),
                    totalSteps = obj.optLong("steps_total", obj.optLong("total_ticks", base.totalSteps))
                )
            } catch (_: Throwable) {}
            debugSnapshot = merged
            delay(200)
        }
    }

    // Surface lifecycle and game loop - Executed on dedicated ReloadedRenderThread (preserves EGL/GLES context!)
    LaunchedEffect(isRunning, isPaused, isSurfaceReady) {
        if (!isRunning || isPaused || !isSurfaceReady) return@LaunchedEffect

        withContext(renderDispatcher) {
            val pacer = StrictStepPacer()
            pacer.reset()

            var frameCount = 0
            var fpsTimer = System.nanoTime()
            var currentStepsPerSec = 0
            var lastStepMs = 0f
            var lastSleepMs = 0f

            while (isActive && isRunning && !isPaused && isSurfaceReady) {
                val targetHz = if (speedMultiplier == 2) 60 else 30
                val waitStartNs = System.nanoTime()
                try {
                    pacer.awaitNext(targetHz)
                } catch (_: InterruptedException) {
                    break
                }
                lastSleepMs = (System.nanoTime() - waitStartNs) / 1_000_000f

                val frameStartNs = System.nanoTime()
                val deltaMs = 1000f / targetHz
                NativeBridge.stepFrameSafe(deltaMs)
                val stepElapsedNs = System.nanoTime() - frameStartNs
                lastStepMs = stepElapsedNs / 1_000_000f

                frameCount++
                val now = System.nanoTime()
                val elapsedSinceTimer = now - fpsTimer
                if (elapsedSinceTimer >= 1_000_000_000L) {
                    currentStepsPerSec = (frameCount * 1_000_000_000.0 / elapsedSinceTimer).toInt()
                    frameCount = 0
                    fpsTimer = now
                }

                if (frameCount % 5 == 0) {
                    val prev = liveMetricsRef.get()
                    liveMetricsRef.set(
                        prev.copy(
                            stepsPerSec = currentStepsPerSec,
                            targetFps = targetHz,
                            stepTimeMs = lastStepMs,
                            sleepTimeMs = lastSleepMs,
                            speedMultiplier = speedMultiplier
                        )
                    )
                }
            }
        }
    }

    DisposableEffect(context) {
        val inputManager = context.getSystemService(Context.INPUT_SERVICE) as? InputManager

        fun checkControllers() {
            var found = false
            val deviceIds = inputManager?.inputDeviceIds ?: InputDevice.getDeviceIds()
            for (id in deviceIds) {
                val dev = inputManager?.getInputDevice(id) ?: InputDevice.getDevice(id)
                if (dev != null && !dev.isVirtual) {
                    val sources = dev.sources
                    val isGamepad = (sources and InputDevice.SOURCE_GAMEPAD) == InputDevice.SOURCE_GAMEPAD
                    val isJoystick = (sources and InputDevice.SOURCE_JOYSTICK) == InputDevice.SOURCE_JOYSTICK
                    if (isGamepad || isJoystick) {
                        found = true
                        break
                    }
                }
            }
            hasPhysicalController = found
        }

        checkControllers()

        val deviceListener = object : InputManager.InputDeviceListener {
            override fun onInputDeviceAdded(deviceId: Int) { checkControllers() }
            override fun onInputDeviceRemoved(deviceId: Int) { checkControllers() }
            override fun onInputDeviceChanged(deviceId: Int) { checkControllers() }
        }

        inputManager?.registerInputDeviceListener(deviceListener, null)

        onDispose {
            isRunning = false
            isSurfaceReady = false
            audioPlayer.stop()
            NativeBridge.onSurfaceDestroyedSafe()
            renderDispatcher.close()
            inputManager?.unregisterInputDeviceListener(deviceListener)
        }
    }

    Box(
        modifier = Modifier.fillMaxSize()
    ) {
        // Native SurfaceView rendering Pokémon Reloaded
        AndroidView(
            modifier = Modifier.fillMaxSize(),
            factory = { ctx ->
                SurfaceView(ctx).apply {
                    holder.addCallback(object : SurfaceHolder.Callback {
                        override fun surfaceCreated(holder: SurfaceHolder) {
                            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
                                try {
                                    holder.surface.setFrameRate(
                                        30.0f,
                                        android.view.Surface.FRAME_RATE_COMPATIBILITY_FIXED_SOURCE
                                    )
                                } catch (_: Throwable) {}
                            }
                            val w = width.coerceAtLeast(800)
                            val h = height.coerceAtLeast(480)
                            NativeBridge.attachSurfaceSafe(holder.surface, w, h)
                            NativeBridge.resumeSafe()
                            isSurfaceReady = true
                        }

                        override fun surfaceChanged(
                            holder: SurfaceHolder,
                            format: Int,
                            width: Int,
                            height: Int
                        ) {
                            NativeBridge.onSurfaceChangedSafe(width, height)
                            NativeBridge.resumeSafe()
                        }

                        override fun surfaceDestroyed(holder: SurfaceHolder) {
                            isSurfaceReady = false
                            NativeBridge.onSurfaceDestroyedSafe()
                        }
                    })
                }
            }
        )

        // Touch Controller Overlay (Hidden if physical controller is connected, unless editing layout)
        if (!hasPhysicalController || isLayoutEditing) {
            TouchControllerOverlay(
                isLayoutEditing = isLayoutEditing,
                overlayOpacity = overlayOpacity,
                onOpacityChanged = { overlayOpacity = it },
                modifier = Modifier.fillMaxSize()
            )
        }

        // Diagnostic Telemetry Overlay (Top-Start)
        if (isDebugOverlayVisible) {
            DebugOverlay(
                snapshot = debugSnapshot,
                isExpanded = isDebugOverlayExpanded,
                onToggleExpand = { isDebugOverlayExpanded = !isDebugOverlayExpanded },
                modifier = Modifier
                    .align(Alignment.TopStart)
                    .padding(top = 12.dp, start = 12.dp)
            )
        }

        // Compact game controls, kept away from the center of the game.
        Column(
            modifier = Modifier
                .align(Alignment.TopEnd)
                .padding(top = 12.dp, end = 12.dp),
            horizontalAlignment = Alignment.End
        ) {
            Surface(
                shape = RoundedCornerShape(14.dp),
                color = Color(0xD9161D27),
                border = BorderStroke(1.dp, Color.White.copy(alpha = 0.14f)),
                shadowElevation = 3.dp
            ) {
                Row(
                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(2.dp)
                ) {
                    if (hasPhysicalController && !isHudCollapsed) {
                        Text(
                            text = "Mando",
                            fontSize = 11.sp,
                            fontWeight = FontWeight.SemiBold,
                            color = Color(0xFF68A678),
                            modifier = Modifier.padding(horizontal = 6.dp)
                        )
                    }

                    if (!isHudCollapsed) {
                        // Plain text speed control, without decorative chips.
                        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                            listOf(1, 2).forEach { mult ->
                                val selected = speedMultiplier == mult
                                Text(
                                    text = "${mult}×",
                                    modifier = Modifier
                                        .clip(RoundedCornerShape(6.dp))
                                        .clickable {
                                            speedMultiplier = mult
                                            NativeBridge.setSpeedMultiplierSafe(mult)
                                        }
                                        .padding(horizontal = 9.dp, vertical = 7.dp),
                                    fontSize = 12.sp,
                                    fontWeight = if (selected) FontWeight.Bold else FontWeight.Medium,
                                    color = if (selected) Color.White else Color.White.copy(alpha = 0.62f)
                                )
                            }
                        }

                        IconButton(
                            onClick = { isDebugOverlayVisible = !isDebugOverlayVisible },
                            modifier = Modifier.size(28.dp)
                        ) {
                            Text(
                                text = "DBG",
                                fontSize = 11.sp,
                                fontWeight = FontWeight.Bold,
                                color = if (isDebugOverlayVisible) Color(0xFF68A678) else Color.White.copy(alpha = 0.62f)
                            )
                        }

                        IconButton(
                            onClick = { isMacroBarVisible = !isMacroBarVisible },
                            modifier = Modifier.size(28.dp)
                        ) {
                            Icon(
                                imageVector = Icons.Default.Keyboard,
                                contentDescription = "Macros F1-F12",
                                tint = if (isMacroBarVisible) Color.White else Color.White.copy(alpha = 0.72f),
                                modifier = Modifier.size(18.dp)
                            )
                        }

                        IconButton(
                            onClick = { isLayoutEditing = !isLayoutEditing },
                            modifier = Modifier.size(28.dp)
                        ) {
                            Icon(
                                imageVector = Icons.Default.DashboardCustomize,
                                contentDescription = "Editar Controles",
                                tint = if (isLayoutEditing) Color.White else Color.White.copy(alpha = 0.72f),
                                modifier = Modifier.size(18.dp)
                            )
                        }

                        IconButton(
                            onClick = {
                                isPaused = true
                                NativeBridge.pauseSafe()
                                audioPlayer.pause()
                                showPauseDialog = true
                            },
                            modifier = Modifier.size(28.dp)
                        ) {
                            Icon(
                                imageVector = Icons.Default.Pause,
                                contentDescription = "Pausa",
                                tint = Color.White,
                                modifier = Modifier.size(18.dp)
                            )
                        }
                    }

                    IconButton(
                        onClick = { isHudCollapsed = !isHudCollapsed },
                        modifier = Modifier.size(24.dp)
                    ) {
                        Icon(
                            imageVector = if (isHudCollapsed) Icons.Default.ChevronRight else Icons.Default.ChevronLeft,
                            contentDescription = "Expandir/Colapsar",
                            tint = Color.White.copy(alpha = 0.78f),
                            modifier = Modifier.size(16.dp)
                        )
                    }
                }
            }

            // Macro Bar (F1-F12 Quick Buttons for Pokémon Reloaded Field Skills)
            AnimatedVisibility(visible = isMacroBarVisible) {
                Surface(
                    modifier = Modifier
                        .padding(top = 8.dp)
                        .clip(RoundedCornerShape(10.dp)),
                    color = Color(0xD9161D27),
                    border = BorderStroke(1.dp, Color.White.copy(alpha = 0.14f)),
                    shadowElevation = 2.dp
                ) {
                    val macroRowScroll = rememberScrollState()
                    Row(
                        modifier = Modifier
                            .horizontalScroll(macroRowScroll)
                            .padding(horizontal = 8.dp, vertical = 6.dp),
                        horizontalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        val macros = listOf(
                            "F1 (Bici)" to InputBits.BTN_F1,
                            "F2 (Caña)" to InputBits.BTN_F2,
                            "F3 (Radar)" to InputBits.BTN_F3,
                            "F4 (PokéGear)" to InputBits.BTN_F4,
                            "F5 (Mapa)" to InputBits.BTN_F5,
                            "F6" to InputBits.BTN_F6,
                            "F7" to InputBits.BTN_F7,
                            "F8" to InputBits.BTN_F8,
                            "F9" to InputBits.BTN_F9,
                            "F10" to InputBits.BTN_F10,
                            "F11" to InputBits.BTN_F11,
                            "F12" to InputBits.BTN_F12
                        )
                        macros.forEach { (label, bit) ->
                            Button(
                                onClick = {
                                    NativeBridge.setButtonStateSafe(bit, true)
                                    NativeBridge.setButtonStateSafe(bit, false)
                                },
                                shape = RoundedCornerShape(6.dp),
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = Color.White.copy(alpha = 0.12f),
                                    contentColor = Color.White
                                ),
                                contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                                modifier = Modifier.height(30.dp),
                                elevation = ButtonDefaults.buttonElevation(defaultElevation = 0.dp)
                            ) {
                                Text(text = label, fontSize = 11.sp, fontWeight = FontWeight.SemiBold)
                            }
                        }
                    }
                }
            }
        }

        // Pause Menu Dialog
        if (showPauseDialog) {
            AlertDialog(
                onDismissRequest = {
                    showPauseDialog = false
                    isPaused = false
                    NativeBridge.resumeSafe()
                    audioPlayer.resume()
                },
                title = {
                    Text(
                        text = "Juego Pausado",
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.Bold
                    )
                },
                text = {
                    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                            Text(
                                text = "El juego está en pausa.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = TextMutedSlate
                        )
                    }
                },
                confirmButton = {
                    Button(
                        onClick = {
                            showPauseDialog = false
                            isPaused = false
                            NativeBridge.resumeSafe()
                            audioPlayer.resume()
                        },
                        colors = ButtonDefaults.buttonColors(
                            containerColor = SteelBlue,
                            contentColor = SurfacePureWhite
                        ),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        Text(text = "Reanudar")
                    }
                },
                dismissButton = {
                    OutlinedButton(
                        onClick = {
                            showPauseDialog = false
                            isRunning = false
                            audioPlayer.stop()
                            NativeBridge.onSurfaceDestroyedSafe()
                            onExitToLauncher()
                        },
                        border = BorderStroke(1.dp, BorderCrisp),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        Text(text = "Salir al Lanzador", color = TextDeepSlate)
                    }
                },
                containerColor = SurfacePureWhite,
                shape = RoundedCornerShape(16.dp)
            )
        }
    }
}

@Composable
fun DebugOverlay(
    snapshot: DebugMetricsSnapshot,
    isExpanded: Boolean,
    onToggleExpand: () -> Unit,
    modifier: Modifier = Modifier
) {
    Surface(
        modifier = modifier.widthIn(min = 190.dp, max = 250.dp),
        shape = RoundedCornerShape(12.dp),
        color = Color(0xF7FFFFFF),
        border = BorderStroke(1.dp, Color(0xFFCBD5E1)),
        shadowElevation = 4.dp
    ) {
        Column(modifier = Modifier.padding(8.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(6.dp))
                    .clickable { onToggleExpand() }
                    .padding(horizontal = 4.dp, vertical = 2.dp)
            ) {
                val isPacingGood = kotlin.math.abs(snapshot.stepsPerSec - snapshot.targetFps) <= 3
                val fpsColor = if (isPacingGood) Color(0xFF2E7D32) else Color(0xFFC62828)

                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = "DIAGNOSTICO",
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        color = Color(0xFF0F172A)
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                    Text(
                        text = "${snapshot.stepsPerSec} fps",
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        color = fpsColor
                    )
                }

                Icon(
                    imageVector = if (isExpanded) Icons.Default.ChevronLeft else Icons.Default.ChevronRight,
                    contentDescription = if (isExpanded) "Minimizar" else "Expandir",
                    tint = Color(0xFF475569),
                    modifier = Modifier.size(16.dp)
                )
            }

            if (isExpanded) {
                Spacer(modifier = Modifier.height(6.dp))
                Column(
                    verticalArrangement = Arrangement.spacedBy(3.dp),
                    modifier = Modifier.padding(horizontal = 4.dp)
                ) {
                    val roomDisplay = if (snapshot.roomName.isNotEmpty()) {
                        "${snapshot.roomName} (#${snapshot.roomId})"
                    } else {
                        "Sala #${snapshot.roomId}"
                    }
                    DebugMetricRow("Objetivo", "${snapshot.targetFps} Hz (${snapshot.speedMultiplier}x)")
                    DebugMetricRow("Sala", roomDisplay)
                    DebugMetricRow("Vel. Sala", "${snapshot.roomSpeed} fps")
                    DebugMetricRow("Paso CPU", "%.1f ms".format(snapshot.stepTimeMs))
                    DebugMetricRow("Espera", "%.1f ms".format(snapshot.sleepTimeMs))
                    DebugMetricRow("Instancias", "${snapshot.instanceCount}")
                    DebugMetricRow("Pasos Tot", "${snapshot.totalSteps}")
                }
            }
        }
    }
}

@Composable
private fun DebugMetricRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = label,
            fontSize = 10.sp,
            color = Color(0xFF64748B),
            fontWeight = FontWeight.Medium
        )
        Text(
            text = value,
            fontSize = 10.sp,
            color = Color(0xFF0F172A),
            fontWeight = FontWeight.SemiBold
        )
    }
}
