package com.reloaded.android.ui.theme

import android.app.Activity
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.unit.dp
import androidx.core.view.WindowCompat

// STRICT LIGHT MODE FIRST: Default color scheme for enterprise clean aesthetic
private val LightColorScheme = lightColorScheme(
    primary = SteelBlue,
    onPrimary = SurfacePureWhite,
    primaryContainer = SteelBlueLight,
    onPrimaryContainer = SteelBlueDark,

    secondary = SlatePastel,
    onSecondary = SurfacePureWhite,
    secondaryContainer = SurfaceUltraLightSlate,
    onSecondaryContainer = TextSoftCharcoal,

    tertiary = SageGreen,
    onTertiary = SurfacePureWhite,
    tertiaryContainer = SageGreenLight,
    onTertiaryContainer = TextDeepSlate,

    background = SurfacePorcelain,
    onBackground = TextDeepSlate,

    surface = SurfacePureWhite,
    onSurface = TextDeepSlate,
    surfaceVariant = SurfaceUltraLightSlate,
    onSurfaceVariant = TextMutedSlate,

    outline = BorderCrisp,
    outlineVariant = BorderSubtle,

    error = MutedCoral,
    onError = SurfacePureWhite,
    errorContainer = MutedCoralLight,
    onErrorContainer = TextDeepSlate
)

// Optional dark scheme only used if explicitly toggled by user (never default)
private val DarkColorScheme = darkColorScheme(
    primary = DarkPrimary,
    onPrimary = DarkSurfaceBackground,
    primaryContainer = DarkSurfaceElevated,
    onPrimaryContainer = DarkTextPrimary,

    secondary = SlatePastel,
    onSecondary = SurfacePureWhite,
    secondaryContainer = DarkSurfaceCard,
    onSecondaryContainer = DarkTextPrimary,

    tertiary = SageGreen,
    onTertiary = SurfacePureWhite,
    tertiaryContainer = DarkSurfaceElevated,
    onTertiaryContainer = DarkTextPrimary,

    background = DarkSurfaceBackground,
    onBackground = DarkTextPrimary,

    surface = DarkSurfaceCard,
    onSurface = DarkTextPrimary,
    surfaceVariant = DarkSurfaceElevated,
    onSurfaceVariant = DarkTextSecondary,

    outline = DarkBorderCrisp,
    outlineVariant = DarkBorderCrisp,

    error = MutedCoral,
    onError = SurfacePureWhite,
    errorContainer = DarkSurfaceElevated,
    onErrorContainer = DarkTextPrimary
)

val AppShapes = Shapes(
    extraSmall = RoundedCornerShape(4.dp),
    small = RoundedCornerShape(8.dp),
    medium = RoundedCornerShape(12.dp),
    large = RoundedCornerShape(16.dp),
    extraLarge = RoundedCornerShape(24.dp)
)

@Composable
fun PokemonReloadedTheme(
    // MANDATORY RULE: Light Mode is strictly default false; never default to isSystemInDarkTheme()
    darkTheme: Boolean = false,
    content: @Composable () -> Unit
) {
    val colorScheme: ColorScheme = if (darkTheme) DarkColorScheme else LightColorScheme

    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as? Activity)?.window
            if (window != null) {
                window.statusBarColor = colorScheme.surface.toArgb()
                window.navigationBarColor = colorScheme.surface.toArgb()
                val controller = WindowCompat.getInsetsController(window, view)
                // When darkTheme is false, use dark status bar icons (light mode background)
                controller.isAppearanceLightStatusBars = !darkTheme
                controller.isAppearanceLightNavigationBars = !darkTheme
            }
        }
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = AppTypography,
        shapes = AppShapes,
        content = content
    )
}
