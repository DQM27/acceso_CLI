package com.brisas.controlacceso

import android.provider.Settings
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.withFrameNanos
import java.time.LocalDate
import java.time.Month
import kotlin.math.PI
import kotlin.math.sin
import kotlin.random.Random

/// Nieve muy sutil sobre la barra de arriba en temporada navideña (pedido
/// del usuario 2026-10-29): los copos caen por la franja del título y los
/// botones, pasan apenas sobre la tarjeta más cercana y se desvanecen ahí.
/// Va encima de todo (no detrás) para poder pisar esa tarjeta, que es opaca.
///
/// - Sólo en temporada ([esTemporadaNavidena]): se prende y se apaga sola,
///   sin sacar versión.
/// - Con "Quitar animaciones" del sistema (escala 0) no se dibuja.
/// - Pocos copos, chicos y transparentes; no recibe toques (un `Canvas` sin
///   `pointerInput` deja pasar el dedo a lo de abajo).
/// - El tiempo se lee sólo al dibujar: cada frame repinta este `Canvas` y
///   nada más, sin recomponer la barra ni la lista.
///
/// Sin [altoBarra] cubre la pantalla entera, detrás del contenido (el login:
/// se pasa rápido y no tiene nada que tapar), con más copos y bajando un
/// poco más rápido para que la caída larga no se vea vacía.
@Composable
internal fun NieveNavidena(altoBarra: Dp? = null, modifier: Modifier = Modifier) {
    val contexto = LocalContext.current
    val animacionesApagadas = remember(contexto) {
        Settings.Global.getFloat(contexto.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f
    }
    // En debug siempre, para poder verla fuera de temporada.
    if (animacionesApagadas || !(BuildConfig.DEBUG || esTemporadaNavidena(LocalDate.now()))) return

    val oscuro = MaterialTheme.colorScheme.surface.luminance() < 0.5f
    // En claro la nieve blanca no se ve: gris azulado tenue.
    val color = if (oscuro) Color.White else Color(0xFF8FA3BF)
    val alfaMaxima = if (oscuro) 0.55f else 0.6f
    val pantallaCompleta = altoBarra == null
    val copos = remember(pantallaCompleta) {
        if (pantallaCompleta) crearCopos(cantidad = 45, segundosMin = 22f, segundosMax = 40f)
        else crearCopos(cantidad = 22, segundosMin = 9f, segundosMax = 18f)
    }
    val segundos = remember { mutableFloatStateOf(0f) }
    LaunchedEffect(Unit) {
        val inicio = withFrameNanos { it }
        // Los frames se detienen solos con la app en segundo plano.
        while (true) {
            withFrameNanos { ahora -> segundos.floatValue = (ahora - inicio) / 1_000_000_000f }
        }
    }

    val tamano = if (altoBarra == null) Modifier.fillMaxSize() else Modifier.fillMaxWidth().height(altoBarra + DESBORDE_NIEVE)
    Canvas(modifier = modifier.then(tamano)) {
        val t = segundos.floatValue
        // En pantalla completa se apaga en el último 15 %, sobre el teclado
        // o el botón de entrar, en vez de cortarse contra el borde.
        val finBarra = altoBarra?.toPx() ?: (size.height * 0.85f)
        val desborde = if (altoBarra == null) size.height - finBarra else DESBORDE_NIEVE.toPx()
        val recorrido = size.height + 8.dp.toPx()
        for (copo in copos) {
            // Arranca un poco por encima del borde para no aparecer de golpe.
            val y = ((copo.fase + t * copo.velocidad) % 1f) * recorrido - 4.dp.toPx()
            val x = (copo.x + copo.vaivenAncho * sin(t * copo.vaivenRitmo + copo.fase * 2f * PI.toFloat())) * size.width
            val alfa = copo.alfa * alfaMaxima * desvanecer(y, finBarra, desborde)
            if (alfa <= 0f) continue
            drawCircle(color.copy(alpha = alfa), radius = copo.radio.dp.toPx(), center = Offset(x, y))
        }
    }
}

/// Del 1 de diciembre al 6 de enero (Reyes) inclusive.
internal fun esTemporadaNavidena(fecha: LocalDate): Boolean =
    (fecha.month == Month.DECEMBER) || (fecha.month == Month.JANUARY && fecha.dayOfMonth <= 6)

/// Opacidad según la altura: entera sobre la barra, se apaga a lo largo del
/// [desborde] (lo poco que pisa la primera tarjeta) y entra suave arriba.
internal fun desvanecer(y: Float, finBarra: Float, desborde: Float): Float = when {
    y < 0f -> 0f
    y < ENTRADA_PX -> y / ENTRADA_PX
    y <= finBarra -> 1f
    desborde <= 0f -> 0f
    else -> (1f - (y - finBarra) / desborde).coerceIn(0f, 1f)
}

private class Copo(
    val x: Float,
    val fase: Float,
    /// Recorridos completos por segundo.
    val velocidad: Float,
    val radio: Float,
    val alfa: Float,
    val vaivenAncho: Float,
    val vaivenRitmo: Float,
)

/// Semilla fija: los mismos copos cada vez, sin saltos al volver a la pantalla.
private fun crearCopos(cantidad: Int, segundosMin: Float, segundosMax: Float): List<Copo> {
    val azar = Random(1225)
    return List(cantidad) {
        Copo(
            x = azar.nextFloat(),
            fase = azar.nextFloat(),
            velocidad = 1f / (segundosMin + azar.nextFloat() * (segundosMax - segundosMin)),
            radio = 1f + azar.nextFloat() * 1.6f,
            alfa = 0.35f + azar.nextFloat() * 0.65f,
            vaivenAncho = 0.005f + azar.nextFloat() * 0.015f,
            vaivenRitmo = 0.4f + azar.nextFloat() * 0.6f,
        )
    }
}

private const val ENTRADA_PX = 12f

/// Cuánto baja la nieve por debajo de la barra: apenas el borde de arriba de
/// la primera tarjeta.
private val DESBORDE_NIEVE = 28.dp
