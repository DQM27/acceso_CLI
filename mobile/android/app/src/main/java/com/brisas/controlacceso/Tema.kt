package com.brisas.controlacceso

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable

/// `GestorTema.temaForzado` (null = seguir al sistema) manda por encima
/// de `isSystemInDarkTheme()` -- ver [temaActual] y el botón de tema en
/// `PantallaPrincipal.kt`.
@Composable
fun TemaBrisas(contenido: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = when (temaActual()) {
            TemaApp.CLARO -> BrisasClaro
            TemaApp.OSCURO -> BrisasOscuro
            TemaApp.TOKYO_NIGHT -> TokyoNight
        },
        shapes = FormasBrisas,
        typography = TipografiaBrisas,
        content = contenido,
    )
}
