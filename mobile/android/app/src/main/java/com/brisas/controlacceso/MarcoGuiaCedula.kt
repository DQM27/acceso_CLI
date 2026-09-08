package com.brisas.controlacceso

import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp

/// Guía liviana de esquinas al estilo "encuadre de escáner".
/// Puramente visual -- sugiere dónde poner el documento, pero
/// [analizarCedula] procesa el frame completo, no solo esta área (ver el
/// doc-comment de esa función: filtrar por este recuadro volvía el escaneo
/// incómodo sin acelerarlo de verdad). Proporción 1.586:1, la misma de una
/// tarjeta ID-1 (cédula/carnet), no un cuadrado genérico.
///
/// En su propio archivo (separado de `PantallaEscanearCedula.kt`) porque es
/// puramente dibujo de UI -- no conoce cámara, ML Kit, ni el estado de
/// escaneo, sólo un color. Mezclarlo con la orquestación de cámara en el
/// mismo archivo no aportaba nada y hacía ese archivo más grande de lo que
/// necesitaba ser.
@Composable
fun MarcoGuiaCedula(color: Color, areaTexto: AreaTextoOcr? = null, modifier: Modifier = Modifier) {
    Canvas(modifier = modifier) {
        val anchoMarco = size.width * 0.94f
        val altoMarco = anchoMarco / 1.586f
        val izquierda = (size.width - anchoMarco) / 2f
        val arriba = (size.height - altoMarco) / 2f
        val radio = CornerRadius(20.dp.toPx())

        drawRoundRect(
            color = Color.White.copy(alpha = 0.45f),
            topLeft = Offset(izquierda, arriba),
            size = Size(anchoMarco, altoMarco),
            cornerRadius = radio,
            style = Stroke(width = 1.5.dp.toPx()),
        )

        if (areaTexto != null) {
            val margen = 18.dp.toPx()
            val izquierdaTexto = (areaTexto.izquierda * size.width - margen).coerceIn(0f, size.width)
            val arribaTexto = (areaTexto.arriba * size.height - margen).coerceIn(0f, size.height)
            val derechaTexto = (areaTexto.derecha * size.width + margen).coerceIn(0f, size.width)
            val abajoTexto = (areaTexto.abajo * size.height + margen).coerceIn(0f, size.height)
            val anchoTexto = derechaTexto - izquierdaTexto
            val altoTexto = abajoTexto - arribaTexto

            if (anchoTexto > 0f && altoTexto > 0f) {
                drawRoundRect(
                    color = color.copy(alpha = 0.9f),
                    topLeft = Offset(izquierdaTexto, arribaTexto),
                    size = Size(anchoTexto, altoTexto),
                    cornerRadius = CornerRadius(14.dp.toPx()),
                    style = Stroke(
                        width = 2.5.dp.toPx(),
                        pathEffect = PathEffect.dashPathEffect(floatArrayOf(18.dp.toPx(), 10.dp.toPx())),
                    ),
                )
            }
        }

        // Cuatro esquinas acentuadas, más gruesas que el borde fino de
        // arriba -- lo que el ojo realmente sigue al alinear la cédula.
        val largoEsquina = 26.dp.toPx()
        val grosor = 4.dp.toPx()
        val derecha = izquierda + anchoMarco
        val abajo = arriba + altoMarco
        val esquinas = listOf(
            // (vértice, hacia la derecha en X, hacia abajo en Y)
            Triple(Offset(izquierda, arriba), true, true),
            Triple(Offset(derecha, arriba), false, true),
            Triple(Offset(izquierda, abajo), true, false),
            Triple(Offset(derecha, abajo), false, false),
        )
        for ((vertice, haciaDerecha, haciaAbajo) in esquinas) {
            val dx = if (haciaDerecha) largoEsquina else -largoEsquina
            val dy = if (haciaAbajo) largoEsquina else -largoEsquina
            drawLine(
                color = color,
                start = vertice,
                end = Offset(vertice.x + dx, vertice.y),
                strokeWidth = grosor,
                cap = StrokeCap.Round,
            )
            drawLine(
                color = color,
                start = vertice,
                end = Offset(vertice.x, vertice.y + dy),
                strokeWidth = grosor,
                cap = StrokeCap.Round,
            )
        }
    }
}
