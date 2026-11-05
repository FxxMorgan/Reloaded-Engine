package com.reloaded.android.ui.game

import android.view.KeyEvent
import android.view.MotionEvent
import com.reloaded.android.bridge.NativeBridge
import com.reloaded.android.bridge.NativeBridge.InputBits

/**
 * High-performance hardware controller mapper for Xbox, PlayStation DualSense, GameSir,
 * 8BitDo, and generic Bluetooth/USB gamepads.
 * Zero-allocation in input dispatch path.
 */
object GamepadMapper {
    private const val ANALOG_DEADZONE = 0.30f

    // Cached states for axis threshold transitions
    private var axisLeftPressed = false
    private var axisRightPressed = false
    private var axisUpPressed = false
    private var axisDownPressed = false

    /**
     * Handles hardware controller key events (D-pad and buttons).
     * Returns true if event was consumed.
     */
    fun handleKeyEvent(event: KeyEvent): Boolean {
        val pressed = event.action == KeyEvent.ACTION_DOWN
        val keyCode = event.keyCode

        val buttonBit = when (keyCode) {
            // D-Pad
            KeyEvent.KEYCODE_DPAD_UP -> InputBits.BTN_UP
            KeyEvent.KEYCODE_DPAD_DOWN -> InputBits.BTN_DOWN
            KeyEvent.KEYCODE_DPAD_LEFT -> InputBits.BTN_LEFT
            KeyEvent.KEYCODE_DPAD_RIGHT -> InputBits.BTN_RIGHT

            // Main Face Buttons (Xbox A/B/X/Y or PS Cross/Circle/Square/Triangle)
            KeyEvent.KEYCODE_BUTTON_A, KeyEvent.KEYCODE_Z -> InputBits.BTN_Z
            KeyEvent.KEYCODE_BUTTON_B, KeyEvent.KEYCODE_X -> InputBits.BTN_X
            KeyEvent.KEYCODE_BUTTON_X, KeyEvent.KEYCODE_C -> InputBits.BTN_C
            KeyEvent.KEYCODE_BUTTON_Y, KeyEvent.KEYCODE_A -> InputBits.BTN_A

            // Shoulder Buttons
            KeyEvent.KEYCODE_BUTTON_L1, KeyEvent.KEYCODE_S -> InputBits.BTN_S
            KeyEvent.KEYCODE_BUTTON_R1, KeyEvent.KEYCODE_D -> InputBits.BTN_D

            // Triggers as buttons
            KeyEvent.KEYCODE_BUTTON_L2 -> InputBits.BTN_SPACE
            KeyEvent.KEYCODE_BUTTON_R2 -> InputBits.BTN_F1

            // System / Menu Buttons
            KeyEvent.KEYCODE_BUTTON_START, KeyEvent.KEYCODE_ENTER -> InputBits.BTN_ENTER
            KeyEvent.KEYCODE_BUTTON_SELECT, KeyEvent.KEYCODE_ESCAPE -> InputBits.BTN_ESCAPE
            KeyEvent.KEYCODE_SPACE -> InputBits.BTN_SPACE

            // Macro / Field Skill Function Keys
            KeyEvent.KEYCODE_F1 -> InputBits.BTN_F1
            KeyEvent.KEYCODE_F2 -> InputBits.BTN_F2
            KeyEvent.KEYCODE_F3 -> InputBits.BTN_F3
            KeyEvent.KEYCODE_F4 -> InputBits.BTN_F4
            KeyEvent.KEYCODE_F5 -> InputBits.BTN_F5
            KeyEvent.KEYCODE_F6 -> InputBits.BTN_F6
            KeyEvent.KEYCODE_F7 -> InputBits.BTN_F7
            KeyEvent.KEYCODE_F8 -> InputBits.BTN_F8
            KeyEvent.KEYCODE_F9 -> InputBits.BTN_F9
            KeyEvent.KEYCODE_F10 -> InputBits.BTN_F10
            KeyEvent.KEYCODE_F11 -> InputBits.BTN_F11
            KeyEvent.KEYCODE_F12 -> InputBits.BTN_F12

            else -> 0
        }

        return if (buttonBit != 0) {
            NativeBridge.setButtonStateSafe(buttonBit, pressed)
            true
        } else {
            false
        }
    }

    /**
     * Handles hardware analog sticks and hat switches (D-pad on some controllers).
     * Returns true if event was consumed.
     */
    fun handleMotionEvent(event: MotionEvent): Boolean {
        if ((event.source and android.view.InputDevice.SOURCE_JOYSTICK) != android.view.InputDevice.SOURCE_JOYSTICK) {
            return false
        }

        // Left Analog Stick & Hat Switch X Axis
        val axisX = event.getAxisValue(MotionEvent.AXIS_X).let { if (it == 0f) event.getAxisValue(MotionEvent.AXIS_HAT_X) else it }
        // Left Analog Stick & Hat Switch Y Axis
        val axisY = event.getAxisValue(MotionEvent.AXIS_Y).let { if (it == 0f) event.getAxisValue(MotionEvent.AXIS_HAT_Y) else it }

        val newLeft = axisX < -ANALOG_DEADZONE
        val newRight = axisX > ANALOG_DEADZONE
        val newUp = axisY < -ANALOG_DEADZONE
        val newDown = axisY > ANALOG_DEADZONE

        if (newLeft != axisLeftPressed) {
            axisLeftPressed = newLeft
            NativeBridge.setButtonStateSafe(InputBits.BTN_LEFT, newLeft)
        }
        if (newRight != axisRightPressed) {
            axisRightPressed = newRight
            NativeBridge.setButtonStateSafe(InputBits.BTN_RIGHT, newRight)
        }
        if (newUp != axisUpPressed) {
            axisUpPressed = newUp
            NativeBridge.setButtonStateSafe(InputBits.BTN_UP, newUp)
        }
        if (newDown != axisDownPressed) {
            axisDownPressed = newDown
            NativeBridge.setButtonStateSafe(InputBits.BTN_DOWN, newDown)
        }

        return true
    }
}
