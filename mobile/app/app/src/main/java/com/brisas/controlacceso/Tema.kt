package com.brisas.controlacceso

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable

@Composable
fun TemaBrisas(contenido: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = if (isSystemInDarkTheme()) BrisasOscuro else BrisasClaro,
        shapes = FormasBrisas,
        typography = TipografiaBrisas,
        content = contenido,
    )
}
