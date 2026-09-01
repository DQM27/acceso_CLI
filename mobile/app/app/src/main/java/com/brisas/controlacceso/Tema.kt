package com.brisas.controlacceso

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

// Misma paleta que desktop/src/index.css (--fondo, --acento, --texto, etc.)
// — un solo lenguaje visual entre escritorio y móvil.
private val FondoClaro = Color(0xFFF6F8F9)
private val AcentoClaro = Color(0xFF087F91)
private val AcentoSuaveClaro = Color(0xFFD9F1F5)
private val TextoClaro = Color(0xFF172026)
private val PanelClaro = Color(0xFFFFFFFF)
private val BordeClaro = Color(0xFFD8E0E5)

private val FondoOscuro = Color(0xFF0A0D0F)
private val AcentoOscuro = Color(0xFF56C8D6)
private val AcentoSuaveOscuro = Color(0xFF1B3238)
private val TextoOscuro = Color(0xFFE8EEF1)
private val PanelOscuro = Color(0xFF12171A)
private val BordeOscuro = Color(0xFF263238)

private val EsquemaClaro =
    lightColorScheme(
        primary = AcentoClaro,
        onPrimary = Color.White,
        primaryContainer = AcentoSuaveClaro,
        onPrimaryContainer = AcentoClaro,
        background = FondoClaro,
        onBackground = TextoClaro,
        surface = PanelClaro,
        onSurface = TextoClaro,
        outline = BordeClaro,
    )

private val EsquemaOscuro =
    darkColorScheme(
        primary = AcentoOscuro,
        onPrimary = Color.Black,
        primaryContainer = AcentoSuaveOscuro,
        onPrimaryContainer = AcentoOscuro,
        background = FondoOscuro,
        onBackground = TextoOscuro,
        surface = PanelOscuro,
        onSurface = TextoOscuro,
        outline = BordeOscuro,
    )

@Composable
fun TemaBrisas(contenido: @Composable () -> Unit) {
    val esquema = if (isSystemInDarkTheme()) EsquemaOscuro else EsquemaClaro
    MaterialTheme(colorScheme = esquema, content = contenido)
}
