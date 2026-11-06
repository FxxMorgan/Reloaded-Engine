package com.reloaded.android

import android.annotation.SuppressLint
import android.content.pm.ActivityInfo
import android.os.Bundle
import android.view.KeyEvent
import android.view.MotionEvent
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.animation.Crossfade
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.reloaded.android.bridge.NativeBridge
import com.reloaded.android.ui.game.GameScreen
import com.reloaded.android.ui.game.GamepadMapper
import com.reloaded.android.ui.launcher.LauncherScreen
import com.reloaded.android.ui.launcher.SaveManagerScreen
import com.reloaded.android.ui.theme.PokemonReloadedTheme

enum class Screen {
    Launcher,
    SaveManager,
    Game
}

class MainActivity : ComponentActivity() {

    private var currentScreen by mutableStateOf(Screen.Launcher)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Initialize Native Bridge storage path
        val storagePath = filesDir.absolutePath
        NativeBridge.initSafe(storagePath)

        setContent {
            val preferences = remember { getSharedPreferences("launcher", MODE_PRIVATE) }
            var isDarkMode by remember { mutableStateOf(preferences.getBoolean("dark_mode", false)) }

            PokemonReloadedTheme(darkTheme = isDarkMode) {
                if (currentScreen == Screen.Game) {
                    setImmersiveGameMode()
                    GameScreen(
                        onExitToLauncher = {
                            updateSystemBarsForUi(isDarkMode)
                            currentScreen = Screen.Launcher
                        }
                    )
                } else {
                    Surface(modifier = Modifier.fillMaxSize()) {
                        Crossfade(targetState = currentScreen, label = "ScreenTransition") { screen ->
                            when (screen) {
                                Screen.Launcher -> {
                                    updateSystemBarsForUi(isDarkMode)
                                    LauncherScreen(
                                        onStartGame = {
                                            setImmersiveGameMode()
                                            currentScreen = Screen.Game
                                        },
                                        onNavigateToSaveManager = {
                                            currentScreen = Screen.SaveManager
                                        },
                                        isDarkMode = isDarkMode,
                                        onToggleDarkMode = {
                                            isDarkMode = it
                                            preferences.edit().putBoolean("dark_mode", it).apply()
                                        }
                                    )
                                }
                                Screen.SaveManager -> {
                                    updateSystemBarsForUi(isDarkMode)
                                    SaveManagerScreen(
                                        onNavigateBack = {
                                            currentScreen = Screen.Launcher
                                        }
                                    )
                                }
                                Screen.Game -> {}
                            }
                        }
                    }
                }
            }
        }
    }

    override fun onPause() {
        if (currentScreen == Screen.Game) GamepadMapper.reset()
        super.onPause()
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (!hasFocus && currentScreen == Screen.Game) GamepadMapper.reset()
    }

    private fun setImmersiveGameMode() {
        requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
        val windowInsetsController = WindowCompat.getInsetsController(window, window.decorView)
        windowInsetsController.systemBarsBehavior =
            WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        windowInsetsController.hide(WindowInsetsCompat.Type.systemBars())
    }

    private fun updateSystemBarsForUi(isDarkMode: Boolean) {
        requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
        val windowInsetsController = WindowCompat.getInsetsController(window, window.decorView)
        windowInsetsController.show(WindowInsetsCompat.Type.systemBars())
        windowInsetsController.isAppearanceLightStatusBars = !isDarkMode
        windowInsetsController.isAppearanceLightNavigationBars = !isDarkMode
    }

    // Public Activity input hook required for controllers before Compose consumes navigation keys.
    // AndroidX marks its implementation restricted; unhandled events still need its dispatch path.
    @SuppressLint("RestrictedApi")
    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        if (currentScreen == Screen.Game) {
            if (GamepadMapper.handleKeyEvent(event)) {
                return true
            }
        }
        return super.dispatchKeyEvent(event)
    }

    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean {
        if (currentScreen == Screen.Game) {
            if (GamepadMapper.handleMotionEvent(event)) {
                return true
            }
        }
        return super.dispatchGenericMotionEvent(event)
    }
}
