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

// Los dos temas de la app son los mismos que en el escritorio (pedido del
// usuario 2026-10-17): Tokyo Day y Tokyo Night, con la paleta de
// `desktop/src/tokyo-day.css` y `desktop/src/tokyo-night.css`. Cada color de
// Material sale del token del escritorio equivalente: fondo -> `--fondo`,
// superficie -> `--panel`, acento -> `--acento`, texto -> `--texto`,
// atenuado -> `--muted`, bordes -> `--borde`/`--borde-fuerte`, y los
// contenedores suaves -> `--acento-suave`/`--info-suave`/`--error-suave`.
//
// Tokyo Day: paleta oficial de folke/tokyonight.nvim (`tokyonight_day`) con
// los textos y estados ajustados en el escritorio para llegar a 4,5:1.
internal val TokyoDay = lightColorScheme(
    primary = Color(0xFF2959AA),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFCFD4E1),
    onPrimaryContainer = Color(0xFF2959AA),
    inversePrimary = Color(0xFF7AA2F7),
    secondary = Color(0xFF2959AA),
    onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFCFD4E1),
    onSecondaryContainer = Color(0xFF2959AA),
    tertiary = Color(0xFF006D84),
    onTertiary = Color(0xFFFFFFFF),
    tertiaryContainer = Color(0xFFCBD6DD),
    onTertiaryContainer = Color(0xFF006D84),
    background = Color(0xFFD0D5E3),
    onBackground = Color(0xFF343B58),
    surface = Color(0xFFE1E2E7),
    onSurface = Color(0xFF343B58),
    surfaceVariant = Color(0xFFC4C8DA),
    onSurfaceVariant = Color(0xFF565F89),
    surfaceTint = Color(0xFF2959AA),
    inverseSurface = Color(0xFF343B58),
    inverseOnSurface = Color(0xFFE1E2E7),
    error = Color(0xFFA93737),
    onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFDBD1D5),
    onErrorContainer = Color(0xFFA93737),
    outline = Color(0xFF848CB5),
    outlineVariant = Color(0xFFA8AECB),
    scrim = Color(0xFFD0D5E3),
    surfaceBright = Color(0xFFE9E9ED),
    surfaceDim = Color(0xFFD0D5E3),
    surfaceContainer = Color(0xFFE1E2E7),
    surfaceContainerHigh = Color(0xFFE9E9ED),
    surfaceContainerHighest = Color(0xFFE9E9ED),
    surfaceContainerLow = Color(0xFFE1E2E7),
    surfaceContainerLowest = Color(0xFFD0D5E3),
)

// Tokyo Night (pedido del usuario 2026-09-24): la paleta del tema de VS Code
// (enkia.tokyo-night), la misma que `desktop/src/tokyo-night.css`.
internal val TokyoNight = darkColorScheme(
    primary = Color(0xFF7AA2F7),
    onPrimary = Color(0xFF16161E),
    primaryContainer = Color(0xFF283457),
    onPrimaryContainer = Color(0xFF7AA2F7),
    inversePrimary = Color(0xFF3D59A1),
    secondary = Color(0xFF7AA2F7),
    onSecondary = Color(0xFF16161E),
    secondaryContainer = Color(0xFF283457),
    onSecondaryContainer = Color(0xFF7AA2F7),
    tertiary = Color(0xFF7DCFFF),
    onTertiary = Color(0xFF16161E),
    tertiaryContainer = Color(0xFF1D3445),
    onTertiaryContainer = Color(0xFF7DCFFF),
    background = Color(0xFF16161E),
    onBackground = Color(0xFFC0CAF5),
    surface = Color(0xFF1A1B26),
    onSurface = Color(0xFFC0CAF5),
    surfaceVariant = Color(0xFF292E42),
    onSurfaceVariant = Color(0xFF9AA5CE),
    surfaceTint = Color(0xFF7AA2F7),
    inverseSurface = Color(0xFFC0CAF5),
    inverseOnSurface = Color(0xFF1A1B26),
    error = Color(0xFFF7768E),
    onError = Color(0xFF16161E),
    errorContainer = Color(0xFF3A2230),
    onErrorContainer = Color(0xFFF7768E),
    outline = Color(0xFF545C7E),
    outlineVariant = Color(0xFF292E42),
    scrim = Color(0xFF16161E),
    surfaceBright = Color(0xFF292E42),
    surfaceDim = Color(0xFF16161E),
    surfaceContainer = Color(0xFF1A1B26),
    surfaceContainerHigh = Color(0xFF24283B),
    surfaceContainerHighest = Color(0xFF292E42),
    surfaceContainerLow = Color(0xFF1A1B26),
    surfaceContainerLowest = Color(0xFF16161E),
)

internal val FormaControlBrisas = RoundedCornerShape(16.dp)
internal val FormasBrisas = Shapes(
    extraSmall = FormaControlBrisas,
    small = FormaControlBrisas,
    medium = RoundedCornerShape(20.dp),
    large = RoundedCornerShape(20.dp),
    extraLarge = RoundedCornerShape(20.dp),
)
// Siguen el tema elegido en la app ([temaActual]), no sólo el del sistema.
internal val ColorRellenoBrisas: Color
    @Composable get() = if (temaActual() == TemaApp.TOKYO_NIGHT) Color(0xFF3D59A1) else Color(0xFF2959AA)
internal val ColorSobreRellenoBrisas: Color
    @Composable get() = Color(0xFFFFFFFF)
internal val EspacioControlBrisas = 12.dp
// 20% más chico que los 48.dp originales, a pedido explícito (2026-09-15) --
// no estaba "muy grande", pero se veía mejor un poco más compacto.
internal val AlturaControlBrisas = 38.dp
internal val ColorExitoBrisas: Color
    @Composable get() = when (temaActual()) {
        TemaApp.TOKYO_DAY -> Color(0xFF4A6330)
        TemaApp.TOKYO_NIGHT -> Color(0xFF9ECE6A)
    }
internal val TipografiaBrisas = Typography(
    bodyLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 14.sp, lineHeight = 21.sp),
    bodyMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 14.sp, lineHeight = 21.sp),
    labelLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 14.sp, fontWeight = FontWeight(700)),
    titleLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 22.sp, fontWeight = FontWeight(700)),
)
