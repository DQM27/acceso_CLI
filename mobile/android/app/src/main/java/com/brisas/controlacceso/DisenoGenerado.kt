// Generado desde design/brisas.json. Editar la fuente y ejecutar node design/generar.mjs.
package com.brisas.controlacceso

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import androidx.compose.material3.Shapes
import androidx.compose.material3.lightColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

internal val BrisasClaro = lightColorScheme(
    primary = Color(0xFF203C63),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFE5EBF4),
    onPrimaryContainer = Color(0xFF203C63),
    inversePrimary = Color(0xFF203C63),
    secondary = Color(0xFF203C63),
    onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFE5EBF4),
    onSecondaryContainer = Color(0xFF203C63),
    tertiary = Color(0xFF426F93),
    onTertiary = Color(0xFFFFFFFF),
    tertiaryContainer = Color(0xFFE7EFF6),
    onTertiaryContainer = Color(0xFF426F93),
    background = Color(0xFFE9EDF3),
    onBackground = Color(0xFF202B3A),
    surface = Color(0xFFFFFFFF),
    onSurface = Color(0xFF202B3A),
    surfaceVariant = Color(0xFFE1E7EF),
    onSurfaceVariant = Color(0xFF536278),
    surfaceTint = Color(0xFF203C63),
    inverseSurface = Color(0xFF202B3A),
    inverseOnSurface = Color(0xFFE9EDF3),
    error = Color(0xFFAD4945),
    onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFF7E8E5),
    onErrorContainer = Color(0xFFAD4945),
    outline = Color(0xFF78879D),
    outlineVariant = Color(0xFFC7D0DD),
    scrim = Color(0xFFE9EDF3),
    surfaceBright = Color(0xFFFFFFFF),
    surfaceDim = Color(0xFFE9EDF3),
    surfaceContainer = Color(0xFFFFFFFF),
    surfaceContainerHigh = Color(0xFFE1E7EF),
    surfaceContainerHighest = Color(0xFFFFFFFF),
    surfaceContainerLow = Color(0xFFF6F8FB),
    surfaceContainerLowest = Color(0xFFE9EDF3),
)

internal val BrisasOscuro = darkColorScheme(
    primary = Color(0xFFA6BCD9),
    onPrimary = Color(0xFF142238),
    primaryContainer = Color(0xFF1B304E),
    onPrimaryContainer = Color(0xFFA6BCD9),
    inversePrimary = Color(0xFFA6BCD9),
    secondary = Color(0xFFA6BCD9),
    onSecondary = Color(0xFF142238),
    secondaryContainer = Color(0xFF1B304E),
    onSecondaryContainer = Color(0xFFA6BCD9),
    tertiary = Color(0xFF93B7D4),
    onTertiary = Color(0xFF142431),
    tertiaryContainer = Color(0xFF263644),
    onTertiaryContainer = Color(0xFF93B7D4),
    background = Color(0xFF0C0F14),
    onBackground = Color(0xFFECF0F6),
    surface = Color(0xFF181C23),
    onSurface = Color(0xFFECF0F6),
    surfaceVariant = Color(0xFF303844),
    onSurfaceVariant = Color(0xFFADB8C8),
    surfaceTint = Color(0xFFA6BCD9),
    inverseSurface = Color(0xFFECF0F6),
    inverseOnSurface = Color(0xFF0C0F14),
    error = Color(0xFFDE9690),
    onError = Color(0xFF301A19),
    errorContainer = Color(0xFF402B2B),
    onErrorContainer = Color(0xFFDE9690),
    outline = Color(0xFF7C8BA2),
    outlineVariant = Color(0xFF424C5B),
    scrim = Color(0xFF0C0F14),
    surfaceBright = Color(0xFF343E4D),
    surfaceDim = Color(0xFF0C0F14),
    surfaceContainer = Color(0xFF181C23),
    surfaceContainerHigh = Color(0xFF303844),
    surfaceContainerHighest = Color(0xFF343E4D),
    surfaceContainerLow = Color(0xFF12161D),
    surfaceContainerLowest = Color(0xFF0C0F14),
)

internal val FormaControlBrisas = RoundedCornerShape(8.dp)
internal val FormasBrisas = Shapes(
    extraSmall = FormaControlBrisas,
    small = FormaControlBrisas,
    medium = RoundedCornerShape(12.dp),
    large = RoundedCornerShape(12.dp),
    extraLarge = RoundedCornerShape(12.dp),
)
internal val ColorRellenoBrisas: Color
    @Composable get() = if (isSystemInDarkTheme()) Color(0xFF203C63) else Color(0xFF203C63)
internal val ColorSobreRellenoBrisas: Color
    @Composable get() = if (isSystemInDarkTheme()) Color(0xFFFFFFFF) else Color(0xFFFFFFFF)
internal val EspacioControlBrisas = 12.dp
internal val AlturaControlBrisas = 48.dp
internal val ColorExitoBrisas: Color
    @Composable get() = if (isSystemInDarkTheme()) Color(0xFF8BC49B) else Color(0xFF35724F)
internal val TipografiaBrisas = Typography(
    bodyLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 14.sp, lineHeight = 21.sp),
    bodyMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 14.sp, lineHeight = 21.sp),
    labelLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 14.sp, fontWeight = FontWeight(600)),
    titleLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 20.sp, fontWeight = FontWeight(600)),
)
