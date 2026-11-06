package com.reloaded.android.ui.game

import android.view.HapticFeedbackConstants
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.reloaded.android.bridge.NativeBridge
import com.reloaded.android.bridge.NativeBridge.InputBits
import com.reloaded.android.ui.theme.BorderCrisp
import com.reloaded.android.ui.theme.SteelBlue
import com.reloaded.android.ui.theme.SurfacePureWhite
import com.reloaded.android.ui.theme.SurfaceUltraLightSlate
import com.reloaded.android.ui.theme.TextMutedSlate
import kotlin.math.atan2
import kotlin.math.roundToInt
import kotlin.math.sqrt

@Composable
fun TouchControllerOverlay(
    isLayoutEditing: Boolean,
    overlayOpacity: Float,
    onOpacityChanged: (Float) -> Unit,
    modifier: Modifier = Modifier
) {
    val view = LocalView.current

    // Drag positions for custom layout
    var dpadOffsetX by remember { mutableFloatStateOf(0f) }
    var dpadOffsetY by remember { mutableFloatStateOf(0f) }
    var actionsOffsetX by remember { mutableFloatStateOf(-30f) }
    var actionsOffsetY by remember { mutableFloatStateOf(0f) }

    // Dynamic Follow-Finger Joystick configuration
    var isDynamicJoystick by remember { mutableStateOf(true) }
    var joystickScale by remember { mutableFloatStateOf(1.0f) }

    Box(
        modifier = modifier
            .fillMaxSize()
            .alpha(if (isLayoutEditing) 1.0f else overlayOpacity)
    ) {
        // Dynamic / Fixed Joystick Touch Zone on Left
        Box(
            modifier = Modifier
                .align(Alignment.BottomStart)
                .offset { IntOffset(dpadOffsetX.roundToInt(), dpadOffsetY.roundToInt()) }
                .then(
                    if (isLayoutEditing) {
                        Modifier.pointerInput(Unit) {
                            detectDragGestures { change, dragAmount ->
                                change.consume()
                                dpadOffsetX += dragAmount.x
                                dpadOffsetY += dragAmount.y
                            }
                        }
                    } else Modifier
                )
        ) {
            FollowFingerJoystick(
                isDynamic = isDynamicJoystick,
                scale = joystickScale,
                isLayoutEditing = isLayoutEditing,
                onDirectionChange = { up, down, left, right ->
                    NativeBridge.setButtonStateSafe(InputBits.BTN_UP, up)
                    NativeBridge.setButtonStateSafe(InputBits.BTN_DOWN, down)
                    NativeBridge.setButtonStateSafe(InputBits.BTN_LEFT, left)
                    NativeBridge.setButtonStateSafe(InputBits.BTN_RIGHT, right)
                },
                onHaptic = {
                    view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY)
                }
            )
        }

        // Center Utility Bar (Escape, Space, Enter)
        Row(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .padding(bottom = 20.dp),
            horizontalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            TouchActionButton(
                label = "ESC",
                bitMask = InputBits.BTN_ESCAPE,
                sizeDp = 44,
                isWide = true,
                onHaptic = { view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY) }
            )
            TouchActionButton(
                label = "SPACE",
                bitMask = InputBits.BTN_SPACE,
                sizeDp = 44,
                isWide = true,
                onHaptic = { view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY) }
            )
            TouchActionButton(
                label = "ENTER",
                bitMask = InputBits.BTN_ENTER,
                sizeDp = 44,
                isWide = true,
                onHaptic = { view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY) }
            )
        }

        // Action Cluster on Right (Z, X, C, A, S, D)
        Box(
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .offset { IntOffset(actionsOffsetX.roundToInt(), actionsOffsetY.roundToInt()) }
                .padding(bottom = 32.dp, end = 20.dp)
                .then(
                    if (isLayoutEditing) {
                        Modifier.pointerInput(Unit) {
                            detectDragGestures { change, dragAmount ->
                                change.consume()
                                actionsOffsetX += dragAmount.x
                                actionsOffsetY += dragAmount.y
                            }
                        }
                    } else Modifier
                )
        ) {
            ActionButtonsCluster(
                onHaptic = { view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY) }
            )
        }

        // Layout Editor Panel overlay when active
        if (isLayoutEditing) {
            Surface(
                modifier = Modifier
                    .align(Alignment.Center)
                    .clip(RoundedCornerShape(16.dp)),
                color = SurfacePureWhite,
                border = BorderStroke(1.dp, BorderCrisp),
                shadowElevation = 8.dp
            ) {
                Column(
                    modifier = Modifier.padding(20.dp),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    Text(
                        text = "Editor de Posición & Controles",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold
                    )
                    Text(
                        text = "Arrastra el D-Pad o los botones a tu gusto en la pantalla",
                        style = MaterialTheme.typography.bodySmall,
                        color = TextMutedSlate
                    )
                    Spacer(modifier = Modifier.height(14.dp))

                    // Joystick Mode Selector (Dynamic vs Fixed)
                    Text(
                        text = "Modo de Joystick",
                        style = MaterialTheme.typography.labelMedium,
                        fontWeight = FontWeight.SemiBold
                    )
                    Spacer(modifier = Modifier.height(6.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Button(
                            onClick = { isDynamicJoystick = true },
                            shape = RoundedCornerShape(8.dp),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = if (isDynamicJoystick) SteelBlue else SurfaceUltraLightSlate,
                                contentColor = if (isDynamicJoystick) SurfacePureWhite else TextMutedSlate
                            ),
                            modifier = Modifier.height(34.dp),
                            contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                            elevation = ButtonDefaults.buttonElevation(defaultElevation = 0.dp)
                        ) {
                            Text(
                                text = "Dinámico (Sigue al dedo)",
                                fontSize = 12.sp,
                                fontWeight = FontWeight.Medium
                            )
                        }

                        Button(
                            onClick = { isDynamicJoystick = false },
                            shape = RoundedCornerShape(8.dp),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = if (!isDynamicJoystick) SteelBlue else SurfaceUltraLightSlate,
                                contentColor = if (!isDynamicJoystick) SurfacePureWhite else TextMutedSlate
                            ),
                            modifier = Modifier.height(34.dp),
                            contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                            elevation = ButtonDefaults.buttonElevation(defaultElevation = 0.dp)
                        ) {
                            Text(
                                text = "Fijo",
                                fontSize = 12.sp,
                                fontWeight = FontWeight.Medium
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(14.dp))

                    // Joystick Scale Slider
                    Text(
                        text = "Tamaño del Joystick: ${(joystickScale * 100).toInt()}%",
                        style = MaterialTheme.typography.labelMedium
                    )
                    Slider(
                        value = joystickScale,
                        onValueChange = { joystickScale = it },
                        valueRange = 0.8f..1.4f,
                        modifier = Modifier.size(width = 250.dp, height = 36.dp),
                        colors = SliderDefaults.colors(
                            thumbColor = SteelBlue,
                            activeTrackColor = SteelBlue
                        )
                    )

                    Spacer(modifier = Modifier.height(10.dp))

                    // Opacity Slider
                    Text(
                        text = "Opacidad de Controles: ${(overlayOpacity * 100).toInt()}%",
                        style = MaterialTheme.typography.labelMedium
                    )
                    Slider(
                        value = overlayOpacity,
                        onValueChange = onOpacityChanged,
                        valueRange = 0.2f..1.0f,
                        modifier = Modifier.size(width = 250.dp, height = 36.dp),
                        colors = SliderDefaults.colors(
                            thumbColor = SteelBlue,
                            activeTrackColor = SteelBlue
                        )
                    )
                }
            }
        }
    }
}

/**
 * Dynamic / Floating Follow-Finger Joystick with Leash Effect
 * When isDynamic == true:
 * - Appears centered right where the thumb lands on touch down
 * - Follows the thumb if dragged beyond maximum radius so direction changes are instantaneous
 * - Returns smoothly to default resting position when released
 */
@Composable
private fun FollowFingerJoystick(
    isDynamic: Boolean,
    scale: Float,
    isLayoutEditing: Boolean,
    onDirectionChange: (up: Boolean, down: Boolean, left: Boolean, right: Boolean) -> Unit,
    onHaptic: () -> Unit
) {
    val density = LocalDensity.current
    val areaWidthDp = 230.dp
    val areaHeightDp = 220.dp
    val areaWidthPx = with(density) { areaWidthDp.toPx() }
    val areaHeightPx = with(density) { areaHeightDp.toPx() }

    val baseRadiusDp = 58.dp * scale
    val baseRadiusPx = with(density) { baseRadiusDp.toPx() }
    val baseSizeDp = baseRadiusDp * 2

    val thumbRadiusDp = 24.dp * scale
    val thumbSizeDp = thumbRadiusDp * 2

    val maxPullPx = 42f * density.density * scale
    val deadZonePx = 12f * density.density * scale

    val defaultCenterX = areaWidthPx * 0.48f
    val defaultCenterY = areaHeightPx * 0.52f

    var baseCenterX by remember { mutableFloatStateOf(defaultCenterX) }
    var baseCenterY by remember { mutableFloatStateOf(defaultCenterY) }
    var thumbOffsetX by remember { mutableFloatStateOf(0f) }
    var thumbOffsetY by remember { mutableFloatStateOf(0f) }
    var isTouching by remember { mutableStateOf(false) }

    Box(
        modifier = Modifier
            .size(width = areaWidthDp, height = areaHeightDp)
            .then(
                if (!isLayoutEditing) {
                    Modifier.pointerInput(isDynamic, scale) {
                        try {
                            awaitPointerEventScope {
                                while (true) {
                                    val event = awaitPointerEvent(PointerEventPass.Main)
                                    val change = event.changes.firstOrNull() ?: continue

                                    if (change.pressed) {
                                        change.consume()
                                        if (!isTouching) {
                                            isTouching = true
                                            onHaptic()
                                            if (isDynamic) {
                                                baseCenterX = change.position.x.coerceIn(baseRadiusPx, areaWidthPx - baseRadiusPx)
                                                baseCenterY = change.position.y.coerceIn(baseRadiusPx, areaHeightPx - baseRadiusPx)
                                                thumbOffsetX = 0f
                                                thumbOffsetY = 0f
                                            }
                                        }

                                        val targetBaseX = if (isDynamic) baseCenterX else defaultCenterX
                                        val targetBaseY = if (isDynamic) baseCenterY else defaultCenterY

                                        val dx = change.position.x - targetBaseX
                                        val dy = change.position.y - targetBaseY
                                        val dist = sqrt(dx * dx + dy * dy)

                                        if (isDynamic && dist > maxPullPx) {
                                            // Follow-finger leash effect: pull the base center along with the finger
                                            val excess = dist - maxPullPx
                                            val newBaseX = (baseCenterX + (dx / dist) * excess)
                                                .coerceIn(baseRadiusPx, areaWidthPx - baseRadiusPx)
                                            val newBaseY = (baseCenterY + (dy / dist) * excess)
                                                .coerceIn(baseRadiusPx, areaHeightPx - baseRadiusPx)
                                            baseCenterX = newBaseX
                                            baseCenterY = newBaseY

                                            thumbOffsetX = (change.position.x - newBaseX).coerceIn(-maxPullPx, maxPullPx)
                                            thumbOffsetY = (change.position.y - newBaseY).coerceIn(-maxPullPx, maxPullPx)
                                        } else if (!isDynamic && dist > maxPullPx) {
                                            thumbOffsetX = (dx / dist) * maxPullPx
                                            thumbOffsetY = (dy / dist) * maxPullPx
                                        } else {
                                            thumbOffsetX = dx
                                            thumbOffsetY = dy
                                        }

                                        val currentDist = sqrt(thumbOffsetX * thumbOffsetX + thumbOffsetY * thumbOffsetY)
                                        if (currentDist > deadZonePx) {
                                            val angle = atan2(thumbOffsetY.toDouble(), thumbOffsetX.toDouble()) * (180.0 / Math.PI)
                                            // 8 directions mapping
                                            val right = angle in -67.5..67.5
                                            val left = angle > 112.5 || angle < -112.5
                                            val down = angle in 22.5..157.5
                                            val up = angle in -157.5..-22.5
                                            onDirectionChange(up, down, left, right)
                                        } else {
                                            onDirectionChange(false, false, false, false)
                                        }
                                    } else {
                                        if (isTouching) {
                                            isTouching = false
                                            thumbOffsetX = 0f
                                            thumbOffsetY = 0f
                                            if (isDynamic) {
                                                baseCenterX = defaultCenterX
                                                baseCenterY = defaultCenterY
                                            }
                                            onDirectionChange(false, false, false, false)
                                        }
                                    }
                                }
                            }
                        } finally {
                            isTouching = false
                            thumbOffsetX = 0f
                            thumbOffsetY = 0f
                            onDirectionChange(false, false, false, false)
                            GamepadMapper.reset()
                        }
                    }
                } else Modifier
            )
    ) {
        val currentBaseX = if (isDynamic) baseCenterX else defaultCenterX
        val currentBaseY = if (isDynamic) baseCenterY else defaultCenterY

        // Joystick Base Ring
        Box(
            modifier = Modifier
                .offset {
                    IntOffset(
                        (currentBaseX - baseRadiusPx).roundToInt(),
                        (currentBaseY - baseRadiusPx).roundToInt()
                    )
                }
                .size(baseSizeDp)
                .clip(CircleShape)
                .background(
                    if (isTouching) Color(0xE624303A)
                    else Color(0xB824303A)
                )
                .border(
                    BorderStroke(1.5.dp, if (isTouching) Color.White.copy(alpha = 0.55f) else Color.White.copy(alpha = 0.24f)),
                    CircleShape
                ),
            contentAlignment = Alignment.Center
        ) {
            // Cross guides for clean Swiss visual feedback
            Box(
                modifier = Modifier
                    .size(width = 2.dp, height = baseSizeDp * 0.55f)
                    .background(BorderCrisp.copy(alpha = 0.7f))
            )
            Box(
                modifier = Modifier
                    .size(width = baseSizeDp * 0.55f, height = 2.dp)
                    .background(BorderCrisp.copy(alpha = 0.7f))
            )

            // Floating thumb stick
            Box(
                modifier = Modifier
                    .offset {
                        IntOffset(
                            thumbOffsetX.roundToInt(),
                            thumbOffsetY.roundToInt()
                        )
                    }
                    .size(thumbSizeDp)
                    .clip(CircleShape)
                    .background(if (isTouching) SteelBlue else Color(0xE6F8FAFC))
                    .border(
                        BorderStroke(1.2.dp, if (isTouching) SteelBlue else Color.White.copy(alpha = 0.5f)),
                        CircleShape
                    )
            )
        }
    }
}

/**
 * Action button cluster matching Pokemon Reloaded's PC combat layout:
 * Z (Attack 1), X (Attack 2), C (Special/Menu), A (Skill), S (Defend), D (Dodge/Dash)
 */
@Composable
private fun ActionButtonsCluster(
    onHaptic: () -> Unit
) {
    Column(
        verticalArrangement = Arrangement.spacedBy(10.dp),
        horizontalAlignment = Alignment.End
    ) {
        // Top row: A, S, D
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            TouchActionButton(label = "A", bitMask = InputBits.BTN_A, sizeDp = 52, onHaptic = onHaptic)
            TouchActionButton(label = "S", bitMask = InputBits.BTN_S, sizeDp = 52, onHaptic = onHaptic)
            TouchActionButton(label = "D", bitMask = InputBits.BTN_D, sizeDp = 52, onHaptic = onHaptic)
        }
        // Bottom row: Z, X, C
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            TouchActionButton(label = "Z", bitMask = InputBits.BTN_Z, sizeDp = 58, isPrimary = true, onHaptic = onHaptic)
            TouchActionButton(label = "X", bitMask = InputBits.BTN_X, sizeDp = 58, isSecondary = true, onHaptic = onHaptic)
            TouchActionButton(label = "C", bitMask = InputBits.BTN_C, sizeDp = 52, onHaptic = onHaptic)
        }
    }
}

@Composable
private fun TouchActionButton(
    label: String,
    bitMask: Int,
    sizeDp: Int,
    isWide: Boolean = false,
    isPrimary: Boolean = false,
    isSecondary: Boolean = false,
    onHaptic: () -> Unit
) {
    var isPressed by remember { mutableStateOf(false) }

    val width = if (isWide) (sizeDp * 1.8).dp else sizeDp.dp
    val height = sizeDp.dp

    val bgColor = when {
        isPressed -> SteelBlue
        isPrimary -> Color(0xE6182625)
        isSecondary -> Color(0xD923303A)
        else -> Color(0xB923303A)
    }

    val textColor = Color.White

    Box(
        modifier = Modifier
            .size(width = width, height = height)
            .clip(if (isWide) RoundedCornerShape(12.dp) else CircleShape)
            .background(bgColor)
            .border(
                BorderStroke(1.2.dp, Color.White.copy(alpha = if (isPressed) 0.6f else 0.22f)),
                if (isWide) RoundedCornerShape(12.dp) else CircleShape
            )
            .pointerInput(bitMask) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    down.consume()
                    isPressed = true
                    onHaptic()
                    NativeBridge.setButtonStateSafe(bitMask, true)

                    try {
                        do {
                            val event = awaitPointerEvent()
                            event.changes.forEach { change ->
                                if (change.id == down.id && change.pressed) {
                                    change.consume()
                                }
                            }
                        } while (event.changes.any { it.id == down.id && it.pressed })
                    } finally {
                        isPressed = false
                        NativeBridge.setButtonStateSafe(bitMask, false)
                    }
                }
            },
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = label,
            fontSize = if (isWide) 11.sp else 16.sp,
            fontWeight = FontWeight.Bold,
            color = textColor
        )
    }
}
