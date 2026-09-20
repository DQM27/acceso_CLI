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
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp

/// Guía visual deliberadamente estática (el rectángulo/esquinas no siguen al
/// documento). Los `boundingBox` de ML Kit están en coordenadas del frame de
/// análisis y `PreviewView.FILL_CENTER` aplica su propio recorte; dibujarlos
/// directamente sobre Compose produce cajas desplazadas según
/// pantalla/orientación. Hasta usar la transformación oficial de CameraX, una
/// guía honesta es preferible a seguimiento falso.
///
/// Rediseñado 2026-09-19 (pedido explícito del usuario: "se ve muy simple
/// para lo que ofrece Compose") -- antes era sólo una línea punteada
/// horizontal con un brillo que la recorría. Ahora sigue el patrón estándar
/// de scanners de documentos (Google ML Kit Document Scanner, apps de
/// verificación de identidad tipo Jumio/Onfido): rectángulo con proporción
/// de cédula real, fondo oscurecido fuera del área de captura (para que el
/// recuadro "salte" en vez de perderse contra el resto de la imagen de
/// cámara), esquinas marcadas en vez de un borde completo, y un barrido
/// vertical dentro del recuadro en vez de horizontal sobre una línea suelta.
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
    // Misma región que de verdad se recorta antes del OCR (ver
    // `RegionGuiaOcr`/`analizarCedula`) -- por defecto la angosta de
    // tarjeta, para que las 3 pantallas que no pasan nada distinto no
    // cambien en nada. El comprobante de carga de ruta pasa
    // `RegionGuiaOcr.DOCUMENTO_ANCHO` para que el recuadro que se ve en
    // pantalla sea el mismo que el que de verdad se analiza -- si no,
    // quedaría un recuadro angosto dibujado encima de un recorte ancho,
    // mintiéndole a quien opera sobre qué parte de la hoja hace falta
    // encuadrar.
    region: RegionGuiaOcr = RegionGuiaOcr.TARJETA_ID,
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
            animation = tween(durationMillis = 1600, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "barridoMarcoOcr",
    )

    Canvas(modifier = modifier) {
        // `region` es la misma fuente de verdad que usa el recorte real
        // antes del OCR -- ver el doc-comment de `RegionGuiaOcr`.
        val anchoGuia = size.width * region.fraccionAncho
        val altoGuia = anchoGuia / region.proporcionAnchoAlto
        val izquierdaGuia = (size.width - anchoGuia) / 2f
        val arribaGuia = size.height * region.fraccionTopCentro - altoGuia / 2f
        val guia = Rect(
            left = izquierdaGuia,
            top = arribaGuia,
            right = izquierdaGuia + anchoGuia,
            bottom = arribaGuia + altoGuia,
        )
        val radioEsquina = 14.dp.toPx()

        // Fondo oscurecido FUERA del recuadro -- 4 rectángulos en vez de un
        // cutout con BlendMode.Clear: mismo resultado visual, sin depender
        // de un `graphicsLayer(compositingStrategy = Offscreen)` extra sólo
        // para esto.
        val colorScrim = Color.Black.copy(alpha = 0.55f)
        drawRect(colorScrim, topLeft = Offset(0f, 0f), size = Size(size.width, guia.top))
        drawRect(colorScrim, topLeft = Offset(0f, guia.bottom), size = Size(size.width, size.height - guia.bottom))
        drawRect(colorScrim, topLeft = Offset(0f, guia.top), size = Size(guia.left, guia.height))
        drawRect(
            colorScrim,
            topLeft = Offset(guia.right, guia.top),
            size = Size(size.width - guia.right, guia.height),
        )

        val colorConPulso = color.copy(alpha = 0.55f + pulso * 0.35f)

        // Borde completo tenue (ayuda a "cerrar" el recuadro visualmente)...
        drawRoundRect(
            color = color.copy(alpha = 0.25f),
            topLeft = guia.topLeft,
            size = guia.size,
            cornerRadius = CornerRadius(radioEsquina),
            style = Stroke(width = 1.5.dp.toPx()),
        )

        // ...más 4 esquinas marcadas (look de scanner de documentos), más
        // gruesas y con el pulso de color -- son las que de verdad guían el
        // ojo hacia dónde encuadrar.
        val largoEsquina = anchoGuia * 0.09f
        val grosorEsquina = 4.dp.toPx()
        fun esquina(origen: Offset, haciaX: Float, haciaY: Float) {
            drawLine(
                color = colorConPulso,
                start = origen,
                end = Offset(origen.x + largoEsquina * haciaX, origen.y),
                strokeWidth = grosorEsquina,
                cap = StrokeCap.Round,
            )
            drawLine(
                color = colorConPulso,
                start = origen,
                end = Offset(origen.x, origen.y + largoEsquina * haciaY),
                strokeWidth = grosorEsquina,
                cap = StrokeCap.Round,
            )
        }
        esquina(Offset(guia.left, guia.top), haciaX = 1f, haciaY = 1f)
        esquina(Offset(guia.right, guia.top), haciaX = -1f, haciaY = 1f)
        esquina(Offset(guia.left, guia.bottom), haciaX = 1f, haciaY = -1f)
        esquina(Offset(guia.right, guia.bottom), haciaX = -1f, haciaY = -1f)

        // Barrido vertical dentro del recuadro -- sólo mientras se busca
        // (una vez confirmado/inválido, un barrido en movimiento contradice
        // el mensaje de "ya terminé de leer").
        if (estado == EstadoEscaneo.BUSCANDO) {
            val yBarrido = guia.top + guia.height * avanceBarrido
            drawLine(
                color = colorConPulso,
                start = Offset(guia.left + 6.dp.toPx(), yBarrido),
                end = Offset(guia.right - 6.dp.toPx(), yBarrido),
                strokeWidth = 2.5.dp.toPx(),
                cap = StrokeCap.Round,
            )
        }
    }
}
