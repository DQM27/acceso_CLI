package com.brisas.controlacceso

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp

/// Guía visual deliberadamente estática. Los `boundingBox` de ML Kit están
/// en coordenadas del frame de análisis y `PreviewView.FILL_CENTER` aplica
/// su propio recorte; dibujarlos directamente sobre Compose produce cajas
/// desplazadas según pantalla/orientación. Hasta usar la transformación
/// oficial de CameraX, una guía honesta es preferible a seguimiento falso.
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

    Canvas(modifier = modifier) {
        val anchoGuia = size.width * 0.72f
        val izquierdaGuia = (size.width - anchoGuia) / 2f
        val yGuia = size.height * 0.52f
        drawLine(
            color = color.copy(alpha = 0.35f + pulso * 0.25f),
            start = Offset(izquierdaGuia, yGuia),
            end = Offset(izquierdaGuia + anchoGuia, yGuia),
            strokeWidth = 2.dp.toPx(),
            pathEffect = PathEffect.dashPathEffect(floatArrayOf(12.dp.toPx(), 12.dp.toPx())),
        )
        val anchoBrillo = anchoGuia * 0.28f
        val inicioBrillo = izquierdaGuia + (anchoGuia - anchoBrillo) * avanceBarrido
        drawLine(
            color = color.copy(alpha = if (estado == EstadoEscaneo.BUSCANDO) 0.72f else 0.48f),
            start = Offset(inicioBrillo, yGuia),
            end = Offset(inicioBrillo + anchoBrillo, yGuia),
            strokeWidth = 3.dp.toPx(),
        )
    }
}
