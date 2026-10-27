package com.brisas.controlacceso

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Login
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/// Hacia dónde va la persona (pedido del usuario 2026-10-27): el ingreso
/// avanza hacia la derecha, como se lee; la salida retrocede hacia la
/// izquierda, como "‹ Volver". [sentido] es el signo del desplazamiento.
internal enum class Paso(val sentido: Float) {
    INGRESO(1f),
    SALIDA(-1f),
}

/// Cuánto tarda la flecha en seguir de largo antes de que la hoja empiece a
/// bajar (ver [IconoPaso]): lo justo para que se vea, sin demorar el cierre.
internal const val MS_ANTES_DE_BAJAR = 90L

/// Lo que espera la flecha antes de entrar: que la hoja ya esté casi arriba.
private const val MS_RETRASO_FLECHA = 120L
private const val MS_FLECHA = 300
private const val MS_REMATE = 200

/// El ícono redondo de las hojas de ingreso y de salida. La flecha entra
/// deslizándose en el sentido del [paso] una sola vez al abrir (nunca en
/// bucle: el operador la ve cientos de veces por turno) y, con [siguiendo]
/// (se confirmó), sigue de largo y se desvanece mientras la hoja baja. Quieta
/// también se entiende: la de ingreso apunta a la puerta, la de salida se
/// aleja de ella (el ícono de salida va en espejo para apuntar a la izquierda).
/// Sólo se mueve en la fase de dibujo: no vuelve a medir ni a componer.
@Composable
internal fun IconoPaso(paso: Paso, siguiendo: Boolean, modifier: Modifier = Modifier) {
    // En unidades de recorrido: -1 = atrás de su lugar, 0 = en su lugar.
    val desplazamiento = remember { Animatable(-paso.sentido) }
    val opacidad = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        delay(MS_RETRASO_FLECHA)
        launch { opacidad.animateTo(1f, tween(MS_FUNDIDO_ENTRADA)) }
        desplazamiento.animateTo(0f, tween(MS_FLECHA, easing = CurvaEntrada))
    }
    LaunchedEffect(siguiendo) {
        if (!siguiendo) return@LaunchedEffect
        launch { opacidad.animateTo(0f, tween(MS_REMATE, easing = CurvaSalida)) }
        desplazamiento.animateTo(paso.sentido * 2, tween(MS_REMATE, easing = CurvaSalida))
    }
    val recorrido = with(LocalDensity.current) { 10.dp.toPx() }
    Box(
        modifier = modifier
            .size(56.dp)
            .clip(CircleShape)
            .background(MaterialTheme.colorScheme.primaryContainer),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            if (paso == Paso.INGRESO) Icons.AutoMirrored.Filled.Login else Icons.AutoMirrored.Filled.Logout,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.primary,
            modifier = Modifier.graphicsLayer {
                translationX = desplazamiento.value * recorrido
                alpha = opacidad.value
                if (paso == Paso.SALIDA) scaleX = -1f
            },
        )
    }
}
