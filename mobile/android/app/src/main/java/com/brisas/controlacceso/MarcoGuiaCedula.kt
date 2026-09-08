package com.brisas.controlacceso

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp

/// Marco dinámico basado en el área donde ML Kit está detectando texto.
/// Puramente visual -- acompaña lo que ya se está leyendo, pero
/// [analizarCedula] procesa el frame completo, no solo esta área (ver el
/// doc-comment de esa función: filtrar por este recuadro volvía el escaneo
/// incómodo sin acelerarlo de verdad).
///
/// En su propio archivo (separado de `PantallaEscanearCedula.kt`) porque es
/// puramente dibujo de UI -- no conoce cámara, ML Kit, ni el estado de
/// escaneo, sólo un color. Mezclarlo con la orquestación de cámara en el
/// mismo archivo no aportaba nada y hacía ese archivo más grande de lo que
/// necesitaba ser.
@Composable
fun MarcoGuiaCedula(
    color: Color,
    estado: EstadoEscaneo,
    areaTexto: AreaTextoOcr? = null,
    modifier: Modifier = Modifier,
) {
    val transicion = rememberInfiniteTransition(label = "marcoOcr")
    val pulso by transicion.animateFloat(
        initialValue = 0.55f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 850, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "pulsoMarcoOcr",
    )
    val avanceBarrido by transicion.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 1250, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "barridoMarcoOcr",
    )
    val faseTrazo by transicion.animateFloat(
        initialValue = 0f,
        targetValue = 28f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 700, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "faseTrazoMarcoOcr",
    )
    val izquierdaAnimada by animateFloatAsState(areaTexto?.izquierda ?: 0.18f, label = "izquierdaMarcoOcr")
    val arribaAnimada by animateFloatAsState(areaTexto?.arriba ?: 0.50f, label = "arribaMarcoOcr")
    val derechaAnimada by animateFloatAsState(areaTexto?.derecha ?: 0.82f, label = "derechaMarcoOcr")
    val abajoAnimada by animateFloatAsState(areaTexto?.abajo ?: 0.54f, label = "abajoMarcoOcr")

    Canvas(modifier = modifier) {
        if (areaTexto != null) {
            val margen = 18.dp.toPx()
            val izquierdaTexto = (izquierdaAnimada * size.width - margen).coerceIn(0f, size.width)
            val arribaTexto = (arribaAnimada * size.height - margen).coerceIn(0f, size.height)
            val derechaTexto = (derechaAnimada * size.width + margen).coerceIn(0f, size.width)
            val abajoTexto = (abajoAnimada * size.height + margen).coerceIn(0f, size.height)
            val anchoTexto = derechaTexto - izquierdaTexto
            val altoTexto = abajoTexto - arribaTexto

            if (anchoTexto > 0f && altoTexto > 0f) {
                val radio = CornerRadius(16.dp.toPx())
                drawRoundRect(
                    color = color.copy(alpha = 0.12f + pulso * 0.08f),
                    topLeft = Offset(izquierdaTexto, arribaTexto),
                    size = Size(anchoTexto, altoTexto),
                    cornerRadius = radio,
                )
                drawRoundRect(
                    color = color.copy(alpha = 0.18f * pulso),
                    topLeft = Offset(izquierdaTexto - 5.dp.toPx(), arribaTexto - 5.dp.toPx()),
                    size = Size(anchoTexto + 10.dp.toPx(), altoTexto + 10.dp.toPx()),
                    cornerRadius = CornerRadius(20.dp.toPx()),
                    style = Stroke(width = 8.dp.toPx()),
                )
                drawRoundRect(
                    color = color.copy(alpha = 0.9f),
                    topLeft = Offset(izquierdaTexto, arribaTexto),
                    size = Size(anchoTexto, altoTexto),
                    cornerRadius = radio,
                    style = Stroke(
                        width = 2.5.dp.toPx(),
                        pathEffect = PathEffect.dashPathEffect(
                            floatArrayOf(18.dp.toPx(), 10.dp.toPx()),
                            faseTrazo,
                        ),
                    ),
                )
                val yBarrido = arribaTexto + altoTexto * avanceBarrido
                drawLine(
                    color = color.copy(alpha = 0.24f + pulso * 0.36f),
                    start = Offset(izquierdaTexto + 10.dp.toPx(), yBarrido),
                    end = Offset(derechaTexto - 10.dp.toPx(), yBarrido),
                    strokeWidth = 3.dp.toPx(),
                )
            }
        } else {
            val anchoGuia = size.width * 0.72f
            val izquierdaGuia = (size.width - anchoGuia) / 2f
            val yGuia = size.height * 0.52f
            drawLine(
                color = Color.White.copy(alpha = 0.28f),
                start = Offset(izquierdaGuia, yGuia),
                end = Offset(izquierdaGuia + anchoGuia, yGuia),
                strokeWidth = 2.dp.toPx(),
                pathEffect = PathEffect.dashPathEffect(floatArrayOf(12.dp.toPx(), 12.dp.toPx())),
            )
            val anchoBrillo = anchoGuia * 0.28f
            val inicioBrillo = izquierdaGuia + (anchoGuia - anchoBrillo) * avanceBarrido
            drawLine(
                color = Color.White.copy(alpha = if (estado == EstadoEscaneo.BUSCANDO) 0.56f else 0.32f),
                start = Offset(inicioBrillo, yGuia),
                end = Offset(inicioBrillo + anchoBrillo, yGuia),
                strokeWidth = 3.dp.toPx(),
            )
        }
    }
}
