package com.brisas.controlacceso

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
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
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/// Hacia dónde va la persona (pedido del usuario 2026-10-27): el ingreso
/// avanza hacia la derecha, como se lee; la salida retrocede hacia la
/// izquierda, como "‹ Volver". [sentido] es el signo del desplazamiento.
internal enum class Paso(val sentido: Float) {
    INGRESO(1f),
    SALIDA(-1f),
}

/// Alto (px) del teclado que estaba a la vista cuando apareció una hoja que
/// sube desde abajo: el del buscador, si se eligió desde ahí; 0 si no había.
///
/// La hoja ya no espera a que ese teclado termine de bajar (2026-11-14):
/// esperaba hasta 600 ms y por eso, eligiendo desde el buscador, tardaba en
/// aparecer, mientras que tocando la lista salía al instante. Ahora sale
/// siempre en el acto y quien la dibuja usa este alto para no acomodarse al
/// teclado que se va: sin eso la hoja (su propia ventana) se armaba sobre la
/// pantalla achicada y después se corría hacia abajo con él (pruebas del
/// usuario 2026-10-29).
@Composable
internal fun rememberAltoTecladoAlAbrir(): Int {
    val ime = WindowInsets.ime
    val densidad = LocalDensity.current
    return remember { ime.getBottom(densidad) }
}

/// Lo que tarda, con margen, en irse el teclado del buscador.
internal const val MS_CIERRE_TECLADO_AJENO = 400L

/// "Persona con ingreso activo" (se escaneó a alguien que ya estaba adentro):
/// una señal de advertencia de tránsito, triángulo amarillo con borde y "!"
/// negros (pedido del usuario 2026-10-27: el triángulo ámbar sobre un círculo
/// pálido no se leía como alerta). Quieta desde 2026-11-14 (fase 3 de
/// `docs/handoff-rediseno-movil-ux.md`): la señal se entiende sin rebote.
/// Se dibuja a mano en un `Canvas`.
@Composable
internal fun SenalAdvertencia(modifier: Modifier = Modifier) {
    Canvas(modifier = modifier.size(width = 64.dp, height = 56.dp)) {
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

/// El ícono redondo de las hojas de ingreso y de salida: la de ingreso
/// apunta a la puerta, la de salida se aleja de ella (el ícono de salida va
/// en espejo para apuntar a la izquierda). Quieto desde 2026-11-14 (fase 3 de
/// `docs/handoff-rediseno-movil-ux.md`): la flecha entraba deslizándose
/// cuando la hoja terminaba de subir, y para eso había que esperar cuadro a
/// cuadro a que se asentara; el operador la ve cientos de veces por turno.
@Composable
internal fun IconoPaso(
    paso: Paso,
    modifier: Modifier = Modifier,
    tamano: Dp = 56.dp,
) {
    Box(
        modifier = modifier
            .size(tamano)
            .clip(CircleShape)
            .background(MaterialTheme.colorScheme.primaryContainer),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            if (paso == Paso.INGRESO) IconosLucide.Ingreso else IconosLucide.Salida,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.primary,
            modifier = Modifier.graphicsLayer {
                if (paso == Paso.SALIDA) scaleX = -1f
            },
        )
    }
}
