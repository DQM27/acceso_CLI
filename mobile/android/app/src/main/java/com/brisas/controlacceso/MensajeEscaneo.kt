package com.brisas.controlacceso

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CenterFocusWeak
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.ErrorOutline
import androidx.compose.material.icons.filled.Schedule
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp

/// De qué tipo es lo que dice la cámara: guía mientras busca, leído bien,
/// algo salió mal, o leído pero vencido.
enum class TonoEscaneo { GUIA, EXITO, ERROR, ADVERTENCIA }

/// El tono del mensaje según el estado del escaneo. [resultadoEsError] es el
/// resultado real de la mutación en el escaneo continuo de gafetes (`null` si
/// no hay): manda sobre el estado de la lectura, que sólo dice si se leyó.
internal fun tonoEscaneo(estado: EstadoEscaneo, vencido: Boolean = false, resultadoEsError: Boolean? = null): TonoEscaneo =
    when {
        resultadoEsError == true -> TonoEscaneo.ERROR
        resultadoEsError == false -> TonoEscaneo.EXITO
        estado == EstadoEscaneo.CONFIRMADO && vencido -> TonoEscaneo.ADVERTENCIA
        estado == EstadoEscaneo.CONFIRMADO -> TonoEscaneo.EXITO
        estado == EstadoEscaneo.INVALIDO -> TonoEscaneo.ERROR
        else -> TonoEscaneo.GUIA
    }

/// El mensaje de las cinco cámaras (cédula, gafete, placa, gafete KOF y
/// comprobante), rediseñado 2026-10-21: una cápsula centrada arriba, con un
/// ícono que dice de un vistazo qué pasó (recuadro mientras busca, check,
/// error, reloj si está vencido) y el fondo del color del estado, los mismos
/// del marco ([ColorEscaneoConfirmado], etc.). Antes era una caja negra de
/// ancho completo que sólo cambiaba de color en el escaneo continuo.
///
/// Barato a propósito, porque convive con la cámara y el OCR: el fondo se
/// funde en la fase de dibujo (`drawBehind`) y el texto sólo se anima cuando
/// cambia, no en cada cuadro. Lo anuncia el lector de pantalla al cambiar.
@Composable
fun MensajeEscaneo(texto: String, tono: TonoEscaneo, modifier: Modifier = Modifier) {
    val fondo by animateColorAsState(colorDeTono(tono), tween(DURACION_FONDO_MS), label = "fondoMensajeEscaneo")
    Row(
        modifier = modifier
            .drawBehind { drawRoundRect(fondo, cornerRadius = CornerRadius(size.height / 2)) }
            .padding(horizontal = 16.dp, vertical = 10.dp)
            .semantics { liveRegion = LiveRegionMode.Polite },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        AnimatedContent(
            targetState = texto to tono,
            transitionSpec = {
                (fadeIn(tween(MS_FUNDIDO_ENTRADA)) + slideInVertically(ResorteMensaje) { alto -> -alto / 3 }) togetherWith
                    fadeOut(tween(MS_SALIDA)) using null
            },
            contentAlignment = Alignment.Center,
            label = "textoMensajeEscaneo",
        ) { (textoMostrado, tonoMostrado) ->
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(iconoDeTono(tonoMostrado), contentDescription = null, tint = Color.White, modifier = Modifier.size(20.dp))
                Spacer(Modifier.width(10.dp))
                Text(
                    textoMostrado,
                    color = Color.White,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = if (tonoMostrado == TonoEscaneo.GUIA) FontWeight.Normal else FontWeight.SemiBold,
                    textAlign = TextAlign.Start,
                )
            }
        }
    }
}

private fun colorDeTono(tono: TonoEscaneo): Color = when (tono) {
    TonoEscaneo.GUIA -> Color.Black.copy(alpha = 0.62f)
    TonoEscaneo.EXITO -> ColorEscaneoConfirmado.copy(alpha = 0.92f)
    TonoEscaneo.ERROR -> ColorEscaneoInvalido.copy(alpha = 0.92f)
    TonoEscaneo.ADVERTENCIA -> ColorEscaneoVencido.copy(alpha = 0.92f)
}

private fun iconoDeTono(tono: TonoEscaneo): ImageVector = when (tono) {
    TonoEscaneo.GUIA -> Icons.Default.CenterFocusWeak
    TonoEscaneo.EXITO -> Icons.Default.CheckCircle
    TonoEscaneo.ERROR -> Icons.Default.ErrorOutline
    TonoEscaneo.ADVERTENCIA -> Icons.Default.Schedule
}

private const val DURACION_FONDO_MS = 200

private val ResorteMensaje = spring<IntOffset>(
    dampingRatio = Spring.DampingRatioMediumBouncy,
    stiffness = Spring.StiffnessMedium,
)
