package com.brisas.controlacceso

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable

/// `GestorTema.oscuroForzado` (null = seguir al sistema) manda por encima
/// de `isSystemInDarkTheme()` -- ver botón claro/oscuro en
/// `PantallaPrincipal.kt`.
@Composable
fun TemaBrisas(contenido: @Composable () -> Unit) {
    val oscuro = GestorTema.oscuroForzado ?: isSystemInDarkTheme()
    MaterialTheme(
        colorScheme = if (oscuro) BrisasOscuro else BrisasClaro,
        shapes = FormasBrisas,
        typography = TipografiaBrisas,
        content = contenido,
    )
}
