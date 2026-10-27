package com.brisas.controlacceso

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
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
/// cámara) y esquinas marcadas en vez de un borde completo.
///
/// Sin animaciones mientras se busca (pedido del usuario 2026-10-27): el
/// barrido vertical que recorría el recuadro no aportaba nada y, como el
/// marco aparecía antes que la imagen, hacía notar la espera de abrir la
/// cámara como una pantalla de carga. Ahora el marco aparece recién con la
/// primera imagen ([visible], ver `EstadoCamaraOcr.vistaPreviaLista`).
///
/// En su propio archivo (separado de `PantallaEscanearCedula.kt`) porque es
/// puramente dibujo de UI -- no conoce cámara, ML Kit, ni el estado de
/// escaneo, sólo un color. Mezclarlo con la orquestación de cámara en el
/// mismo archivo no aportaba nada y hacía ese archivo más grande de lo que
/// necesitaba ser.
@Composable
fun MarcoGuiaCedula(
    color: Color,
    // La cámara ya muestra imagen: hasta ahí no se dibuja nada.
    visible: Boolean,
    modifier: Modifier = Modifier,
    // Misma región que de verdad se recorta antes del OCR (ver
    // `RegionGuiaOcr`/`analizarFrameOcr`) -- por defecto la angosta de
    // tarjeta, para que las 3 pantallas que no pasan nada distinto no
    // cambien en nada. `null` (comprobante de carga de ruta, sin recorte
    // -- ver el comentario en `analizarFrameOcr`) dibuja el recuadro sobre
    // toda la pantalla, sin oscurecer nada: mentirle a quien opera con un
    // recuadro angosto cuando en realidad se está analizando el frame
    // completo sería peor que no mostrar ningún límite.
    region: RegionGuiaOcr? = RegionGuiaOcr.TARJETA_ID,
    // 0..1 -- lecturas coincidentes sobre las necesarias para confirmar
    // (`ResultadoEstabilizacion.progreso`). Se dibuja como barra en el
    // borde inferior del recuadro: "ya lo vi, falta poco, no lo mueva".
    progreso: Float = 0f,
) {
    // El cambio de estado (buscando, confirmado, inválido, vencido) se funde en
    // vez de saltar, y la barra de progreso avanza en vez de dar saltos. Los dos
    // se leen dentro del `Canvas` (fase de dibujo): animarlos no recompone nada.
    val colorMarco by animateColorAsState(color, tween(DURACION_CAMBIO_COLOR_MS), label = "colorMarcoOcr")
    val progresoSuave by animateFloatAsState(progreso, tween(DURACION_AVANCE_PROGRESO_MS), label = "progresoMarcoOcr")

    // Entra con un fundido corto junto con la imagen de la cámara. Quieto el
    // resto del tiempo: sólo se vuelve a dibujar cuando cambia el color o el
    // progreso (antes hubo un pulso y un barrido que redibujaban en cada
    // cuadro, en pantallas que van justas de tiempo con la cámara y el OCR).
    AnimatedVisibility(
        visible = visible,
        modifier = modifier,
        enter = fadeIn(tween(MS_FUNDIDO_ENTRADA)),
        exit = ExitTransition.None,
        label = "marcoOcr",
    ) {
    Canvas(modifier = Modifier.fillMaxSize()) {
        val guia = rectGuia(size, region)
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

        val colorEsquinas = colorMarco.copy(alpha = 0.9f)

        // Borde completo tenue (ayuda a "cerrar" el recuadro visualmente)...
        drawRoundRect(
            color = color.copy(alpha = 0.25f),
            topLeft = guia.topLeft,
            size = guia.size,
            cornerRadius = CornerRadius(radioEsquina),
            style = Stroke(width = 1.5.dp.toPx()),
        )

        // ...más 4 esquinas marcadas (look de scanner de documentos), más
        // gruesas -- son las que de verdad guían el
        // ojo hacia dónde encuadrar.
        val largoEsquina = guia.width * 0.09f
        val grosorEsquina = 4.dp.toPx()
        fun esquina(origen: Offset, haciaX: Float, haciaY: Float) {
            drawLine(
                color = colorEsquinas,
                start = origen,
                end = Offset(origen.x + largoEsquina * haciaX, origen.y),
                strokeWidth = grosorEsquina,
                cap = StrokeCap.Round,
            )
            drawLine(
                color = colorEsquinas,
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

        if (progresoSuave > 0f) {
            val margen = 10.dp.toPx()
            val y = guia.bottom - 8.dp.toPx()
            val anchoUtil = guia.width - 2 * margen
            drawLine(
                color = Color.White.copy(alpha = 0.35f),
                start = Offset(guia.left + margen, y),
                end = Offset(guia.right - margen, y),
                strokeWidth = 5.dp.toPx(),
                cap = StrokeCap.Round,
            )
            drawLine(
                color = colorMarco,
                start = Offset(guia.left + margen, y),
                end = Offset(guia.left + margen + anchoUtil * progresoSuave.coerceIn(0f, 1f), y),
                strokeWidth = 5.dp.toPx(),
                cap = StrokeCap.Round,
            )
        }

    }
    }
}

/// El recuadro guía dentro de un lienzo de [size]. `region` es la misma
/// fuente de verdad que usa el recorte real antes del OCR -- ver el
/// doc-comment de `RegionGuiaOcr`. Sin región (`null`), toda la pantalla.
private fun rectGuia(size: Size, region: RegionGuiaOcr?): Rect {
    val anchoGuia = region?.let { size.width * it.fraccionAncho } ?: size.width
    val altoGuia = region?.let { anchoGuia / it.proporcionAnchoAlto } ?: size.height
    val izquierdaGuia = (size.width - anchoGuia) / 2f
    val arribaGuia = region?.let { size.height * it.fraccionTopCentro - altoGuia / 2f } ?: 0f
    return Rect(left = izquierdaGuia, top = arribaGuia, right = izquierdaGuia + anchoGuia, bottom = arribaGuia + altoGuia)
}

private const val DURACION_CAMBIO_COLOR_MS = 250
private const val DURACION_AVANCE_PROGRESO_MS = 150
