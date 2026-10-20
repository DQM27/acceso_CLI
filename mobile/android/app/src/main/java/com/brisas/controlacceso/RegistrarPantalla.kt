package com.brisas.controlacceso

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.withFrameNanos

/// Marca `nombre` como la pantalla visible mientras esté en la composición
/// (a ella se atribuyen los frames trabados y las muestras de memoria) y
/// mide cuánto tarda en dibujar su primer frame (este último sólo con el grupo
/// `rendimiento`).
@Composable
fun RegistrarPantalla(nombre: String) {
    if (!Telemetria.activa) return
    val inicio = remember(nombre) { System.nanoTime() }
    DisposableEffect(nombre) {
        Telemetria.entrarPantalla(nombre)
        onDispose { Telemetria.salirPantalla(nombre) }
    }
    if (Telemetria.permite("pantalla")) {
        LaunchedEffect(nombre) {
            withFrameNanos { }
            Telemetria.pantallaDibujada(nombre, System.nanoTime() - inicio)
        }
    }
}
