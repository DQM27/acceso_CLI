package com.brisas.controlacceso

import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
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
fun MarcoGuiaCedula(color: Color, areaTexto: AreaTextoOcr? = null, modifier: Modifier = Modifier) {
    Canvas(modifier = modifier) {
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
        }
    }
}
