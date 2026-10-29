package com.brisas.controlacceso

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Login
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SheetState
import androidx.compose.material3.SheetValue
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull

/// Hacia dónde va la persona (pedido del usuario 2026-10-27): el ingreso
/// avanza hacia la derecha, como se lee; la salida retrocede hacia la
/// izquierda, como "‹ Volver". [sentido] es el signo del desplazamiento.
internal enum class Paso(val sentido: Float) {
    INGRESO(1f),
    SALIDA(-1f),
}

/// Lo que dura la entrada de la flecha. La hoja de ingreso abre el teclado
/// recién cuando termina: las dos cosas a la vez se trababan.
internal const val MS_FLECHA = 320

/// `true` desde que la hoja terminó de subir (y ya no vuelve a `false`). La
/// flecha arranca recién ahí: mientras la hoja sube, el ojo sigue a la hoja y
/// la flecha pasaba sin que nadie la viera.
///
/// `currentValue` pasa a `Expanded` a mitad de camino (cuando la hoja ya está
/// más cerca de arriba que de abajo), no al detenerse: por eso, después, se
/// espera cuadro a cuadro a que su posición deje de cambiar. Es sólo el
/// último tramo de la subida y no compone nada.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun rememberHojaArriba(estado: SheetState): Boolean {
    var arriba by remember { mutableStateOf(false) }
    LaunchedEffect(estado) {
        snapshotFlow { estado.currentValue }.first { it == SheetValue.Expanded }
        var anterior = Float.NaN
        // Con tope (medio segundo a 60 cuadros): si la posición no se puede
        // leer, la flecha igual entra.
        for (cuadro in 0 until 30) {
            val actual = runCatching { estado.requireOffset() }.getOrDefault(Float.NaN)
            if (actual == anterior) break
            anterior = actual
            withFrameNanos { }
        }
        arriba = true
    }
    return arriba
}

/// `true` cuando ya no hay teclado a la vista: recién entonces se muestra una
/// hoja que sube desde abajo. Al elegir desde el buscador el teclado sigue
/// bajando, y la hoja (su propia ventana) se acomodaba a la pantalla achicada
/// y después se corría hacia abajo junto con el teclado (pruebas del usuario
/// 2026-10-29). Lo pide cerrar y espera a que termine, con tope: si el
/// teclado no informa su alto, la hoja igual aparece.
@Composable
internal fun rememberTecladoCerrado(): Boolean {
    val ime = WindowInsets.ime
    val densidad = LocalDensity.current
    val teclado = LocalSoftwareKeyboardController.current
    var cerrado by remember { mutableStateOf(ime.getBottom(densidad) == 0) }
    LaunchedEffect(Unit) {
        if (cerrado) return@LaunchedEffect
        teclado?.hide()
        withTimeoutOrNull(MS_ESPERA_TECLADO) { snapshotFlow { ime.getBottom(densidad) }.first { it == 0 } }
        cerrado = true
    }
    return cerrado
}

private const val MS_ESPERA_TECLADO = 600L

/// "Persona con ingreso activo" (se escaneó a alguien que ya estaba adentro):
/// una señal de advertencia de tránsito, triángulo amarillo con borde y "!"
/// negros (pedido del usuario 2026-10-27: el triángulo ámbar sobre un círculo
/// pálido no se leía como alerta). Cuando la hoja ya está [arriba] entra con
/// un rebote, una sola vez. Se dibuja a mano en un `Canvas` y el rebote es
/// sólo escala en la fase de dibujo: no vuelve a medir ni a componer.
@Composable
internal fun SenalAdvertencia(arriba: Boolean, modifier: Modifier = Modifier) {
    val escala = remember { Animatable(0.6f) }
    LaunchedEffect(arriba) {
        if (!arriba) return@LaunchedEffect
        escala.animateTo(1f, spring(dampingRatio = 0.4f, stiffness = Spring.StiffnessMediumLow))
    }
    Canvas(
        modifier = modifier
            .size(width = 64.dp, height = 56.dp)
            .graphicsLayer {
                scaleX = escala.value
                scaleY = escala.value
            },
    ) {
        dibujarSenal()
    }
}

/// La misma señal, chica y quieta, para marcar en una lista a quien ya está
/// adentro: en una lista que se desplaza, el rebote se repetiría con cada fila.
@Composable
internal fun SenalAdentro(modifier: Modifier = Modifier) {
    Canvas(modifier = modifier.size(width = 26.dp, height = 23.dp)) {
        dibujarSenal()
    }
}

/// Triángulo amarillo con borde y "!" negros, del tamaño del lienzo.
private fun DrawScope.dibujarSenal() {
    val ancho = size.width
    val alto = size.height
    // Margen para que el trazo que redondea las puntas no se corte.
    val margen = ancho * 0.07f
    val triangulo = Path().apply {
        moveTo(ancho / 2, margen)
        lineTo(ancho - margen, alto - margen)
        lineTo(margen, alto - margen)
        close()
    }
    val puntas = Stroke(width = ancho * 0.1f, join = StrokeJoin.Round)
    // Borde: el triángulo entero en negro; adentro, el mismo más chico
    // (escalado desde su centro) en amarillo.
    drawPath(triangulo, NegroSenal)
    drawPath(triangulo, NegroSenal, style = puntas)
    val centro = Offset(ancho / 2, (margen + 2 * (alto - margen)) / 3)
    scale(0.76f, pivot = centro) {
        drawPath(triangulo, AmarilloSenal)
        drawPath(triangulo, AmarilloSenal, style = puntas)
    }
    // El "!": un trazo redondeado y un punto.
    drawLine(
        NegroSenal,
        start = Offset(ancho / 2, alto * 0.38f),
        end = Offset(ancho / 2, alto * 0.62f),
        strokeWidth = ancho * 0.085f,
        cap = StrokeCap.Round,
    )
    drawCircle(NegroSenal, radius = ancho * 0.05f, center = Offset(ancho / 2, alto * 0.76f))
}

/// El ícono redondo de las hojas de ingreso y de salida. La flecha entra
/// deslizándose en el sentido del [paso] una sola vez, cuando la hoja ya está
/// [arriba] (nunca en bucle: el operador la ve cientos de veces por turno).
/// Al confirmar no hace nada: la hoja se cierra en el acto y una animación de
/// cierre no se llegaba a ver (se probó y se quitó, 2026-10-27). Quieta
/// también se entiende: la de ingreso apunta a la puerta, la de salida se
/// aleja de ella (el ícono de salida va en espejo para apuntar a la
/// izquierda). Sólo se mueve en la fase de dibujo: no vuelve a medir ni a
/// componer.
@Composable
internal fun IconoPaso(
    paso: Paso,
    arriba: Boolean,
    modifier: Modifier = Modifier,
    tamano: Dp = 56.dp,
) {
    // En unidades de recorrido: -1 = atrás de su lugar, 0 = en su lugar.
    val desplazamiento = remember { Animatable(-paso.sentido) }
    val opacidad = remember { Animatable(0f) }
    LaunchedEffect(arriba) {
        if (!arriba) return@LaunchedEffect
        launch { opacidad.animateTo(1f, tween(MS_FUNDIDO_ENTRADA)) }
        desplazamiento.animateTo(0f, tween(MS_FLECHA, easing = CurvaEntrada))
    }
    val recorrido = with(LocalDensity.current) { 14.dp.toPx() }
    Box(
        modifier = modifier
            .size(tamano)
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
