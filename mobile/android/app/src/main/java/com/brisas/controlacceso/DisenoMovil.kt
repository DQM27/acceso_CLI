// Identidad visual propia de Android (2026-09-15) -- hasta acá este archivo
// era `DisenoGenerado.kt`, generado desde `design/brisas.json` junto con
// desktop y web-visitas (ver `design/generar.mjs`). Se desacopló a pedido
// explícito: el usuario quería el look de la app "Kash" (acento
// verde-azulado, fondo lavanda muy claro, esquinas bien redondeadas) SOLO
// en el teléfono -- desktop sigue con su navy ya publicado (hasta v1.5.4) y
// web-visitas acaba de pasar por un rediseño propio con acento rojo
// (`docs/features-futuras/plan-rediseno-web-visitas.md`); pisarlo con este
// cambio habría sido una regresión de marca no pedida.
//
// A partir de ahora este archivo se edita a mano -- `design/generar.mjs` ya
// no lo toca (ver ese archivo). Si algún día se decide unificar de nuevo la
// marca entre las tres plataformas, hay que sumar `mobile` de vuelta al
// generador en vez de reintroducir un archivo "Generado" que nadie genera.
package com.brisas.controlacceso

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Typography
import androidx.compose.material3.Shapes
import androidx.compose.material3.lightColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import androidx.compose.ui.unit.dp

internal val BrisasClaro = lightColorScheme(
    primary = Color(0xFF0E8F9E),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFD9F0F2),
    onPrimaryContainer = Color(0xFF0E8F9E),
    inversePrimary = Color(0xFF6FD3DD),
    secondary = Color(0xFF0E8F9E),
    onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFD9F0F2),
    onSecondaryContainer = Color(0xFF0E8F9E),
    tertiary = Color(0xFF146C94),
    onTertiary = Color(0xFFFFFFFF),
    tertiaryContainer = Color(0xFFE1EEF5),
    onTertiaryContainer = Color(0xFF146C94),
    background = Color(0xFFF4F3FA),
    onBackground = Color(0xFF1B1B23),
    surface = Color(0xFFFFFFFF),
    onSurface = Color(0xFF1B1B23),
    surfaceVariant = Color(0xFFECEAF6),
    onSurfaceVariant = Color(0xFF6B7280),
    surfaceTint = Color(0xFF0E8F9E),
    inverseSurface = Color(0xFF1B1B23),
    inverseOnSurface = Color(0xFFF4F3FA),
    error = Color(0xFFDC2626),
    onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFFBE7E7),
    onErrorContainer = Color(0xFFDC2626),
    outline = Color(0xFF9CA3AF),
    outlineVariant = Color(0xFFE5E3F1),
    scrim = Color(0xFFF4F3FA),
    surfaceBright = Color(0xFFFFFFFF),
    surfaceDim = Color(0xFFF4F3FA),
    surfaceContainer = Color(0xFFFFFFFF),
    surfaceContainerHigh = Color(0xFFECEAF6),
    surfaceContainerHighest = Color(0xFFFFFFFF),
    surfaceContainerLow = Color(0xFFFFFFFF),
    surfaceContainerLowest = Color(0xFFF4F3FA),
)

internal val BrisasOscuro = darkColorScheme(
    primary = Color(0xFF6FD3DD),
    onPrimary = Color(0xFF06343A),
    primaryContainer = Color(0xFF0B4E56),
    onPrimaryContainer = Color(0xFF6FD3DD),
    inversePrimary = Color(0xFF0E8F9E),
    secondary = Color(0xFF6FD3DD),
    onSecondary = Color(0xFF06343A),
    secondaryContainer = Color(0xFF0B4E56),
    onSecondaryContainer = Color(0xFF6FD3DD),
    tertiary = Color(0xFF8FC2DC),
    onTertiary = Color(0xFF0B2E3F),
    tertiaryContainer = Color(0xFF17475F),
    onTertiaryContainer = Color(0xFF8FC2DC),
    background = Color(0xFF0F1115),
    onBackground = Color(0xFFECEAF6),
    surface = Color(0xFF1B1D24),
    onSurface = Color(0xFFECEAF6),
    surfaceVariant = Color(0xFF2A2D36),
    onSurfaceVariant = Color(0xFFA9AEC0),
    surfaceTint = Color(0xFF6FD3DD),
    inverseSurface = Color(0xFFECEAF6),
    inverseOnSurface = Color(0xFF0F1115),
    error = Color(0xFFF2A29C),
    onError = Color(0xFF3A0E0B),
    errorContainer = Color(0xFF5C1A15),
    onErrorContainer = Color(0xFFF2A29C),
    outline = Color(0xFF6B7280),
    outlineVariant = Color(0xFF3A3D48),
    scrim = Color(0xFF0F1115),
    surfaceBright = Color(0xFF2A2D36),
    surfaceDim = Color(0xFF0F1115),
    surfaceContainer = Color(0xFF1B1D24),
    surfaceContainerHigh = Color(0xFF2A2D36),
    surfaceContainerHighest = Color(0xFF343844),
    surfaceContainerLow = Color(0xFF171920),
    surfaceContainerLowest = Color(0xFF0F1115),
)

internal val FormaControlBrisas = RoundedCornerShape(16.dp)
internal val FormasBrisas = Shapes(
    extraSmall = FormaControlBrisas,
    small = FormaControlBrisas,
    medium = RoundedCornerShape(20.dp),
    large = RoundedCornerShape(20.dp),
    extraLarge = RoundedCornerShape(20.dp),
)
internal val ColorRellenoBrisas: Color
    @Composable get() = if (isSystemInDarkTheme()) Color(0xFF0E8F9E) else Color(0xFF0E8F9E)
internal val ColorSobreRellenoBrisas: Color
    @Composable get() = if (isSystemInDarkTheme()) Color(0xFFFFFFFF) else Color(0xFFFFFFFF)
internal val EspacioControlBrisas = 12.dp
internal val AlturaControlBrisas = 48.dp
internal val ColorExitoBrisas: Color
    @Composable get() = if (isSystemInDarkTheme()) Color(0xFF8BC49B) else Color(0xFF35724F)
internal val TipografiaBrisas = Typography(
    bodyLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 14.sp, lineHeight = 21.sp),
    bodyMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 14.sp, lineHeight = 21.sp),
    labelLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 14.sp, fontWeight = FontWeight(700)),
    titleLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 22.sp, fontWeight = FontWeight(700)),
)
