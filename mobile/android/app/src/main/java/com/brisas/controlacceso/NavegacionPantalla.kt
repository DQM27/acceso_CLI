package com.brisas.controlacceso

import androidx.activity.BackEventCompat
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBackIos
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch

/// Encabezado de toda pantalla que se abre encima de otra (avisos, ayuda,
/// formularios, confirmar ingreso), como en iOS: "‹ Volver" chico arriba a la
/// izquierda y el título grande debajo, con una acción opcional a su derecha.
/// Antes convivían este y un "← Volver" de texto arriba a la derecha en los
/// formularios.
@Composable
internal fun EncabezadoPantalla(
    titulo: String,
    onVolver: () -> Unit,
    modifier: Modifier = Modifier,
    subtitulo: String? = null,
    accion: (@Composable () -> Unit)? = null,
) {
    Column(modifier = modifier.fillMaxWidth()) {
        BotonDiscretoBrisas(onClick = onVolver, modifier = Modifier.offset(x = (-12).dp)) {
            Icon(Icons.AutoMirrored.Filled.ArrowBackIos, contentDescription = null, modifier = Modifier.size(16.dp))
            Text("Volver")
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                titulo,
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.weight(1f),
            )
            accion?.invoke()
        }
        if (subtitulo != null) {
            Text(
                subtitulo,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 2.dp),
            )
        }
    }
}

/// Atrás del sistema con vista previa: mientras se desliza desde el borde, la
/// pantalla se achica y se corre con el dedo, y si se suelta antes de tiempo
/// vuelve a su lugar. Es el equivalente en Android del deslizar para volver
/// de iOS (el manifiesto ya tiene `enableOnBackInvokedCallback`). En Android
/// 13 o anterior funciona como el atrás de siempre.
@Composable
internal fun PantallaConGestoVolver(
    onVolver: () -> Unit,
    modifier: Modifier = Modifier,
    habilitado: Boolean = true,
    contenido: @Composable () -> Unit,
) {
    val avance = remember { Animatable(0f) }
    var desdeIzquierda by remember { mutableStateOf(true) }
    val alcance = rememberCoroutineScope()
    val alVolver by rememberUpdatedState(onVolver)
    PredictiveBackHandler(enabled = habilitado) { eventos ->
        try {
            eventos.collect { evento ->
                desdeIzquierda = evento.swipeEdge == BackEventCompat.EDGE_LEFT
                avance.snapTo(evento.progress)
            }
            alVolver()
        } finally {
            // Si la pantalla sigue (soltó antes de tiempo, o "volver" sólo
            // cambió algo adentro, como en la Ayuda), regresa a su tamaño.
            alcance.launch { avance.animateTo(0f, tween(MS_SALIDA)) }
        }
    }
    Box(
        modifier = modifier
            .fillMaxSize()
            .graphicsLayer {
                val progreso = avance.value
                val escala = 1f - ESCALA_GESTO * progreso
                scaleX = escala
                scaleY = escala
                translationX = (if (desdeIzquierda) 1f else -1f) * progreso * 12.dp.toPx()
                if (progreso > 0f) {
                    shape = RoundedCornerShape((28 * progreso).dp)
                    clip = true
                }
            },
    ) {
        contenido()
    }
}

/// Cuánto se achica la pantalla con el gesto completo (como el sistema).
private const val ESCALA_GESTO = 0.1f
