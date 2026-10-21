package com.brisas.controlacceso

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material3.Icon
import androidx.compose.material3.Surface
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

/// Un registro que acaba de salir bien ("Ingreso registrado · Ana Mora").
/// `id` distingue dos confirmaciones con el mismo texto seguidas: cada una
/// vuelve a mostrarse y a contar su tiempo.
data class Confirmacion(val texto: String, val id: Long)

/// La última confirmación de una pantalla, para [FranjaConfirmacion]. Vive en
/// el ViewModel de cada sección (lo pone quien registró), así un registro que
/// termina mientras la pantalla se recompone no pierde su aviso.
class ConfirmacionReciente {
    var actual by mutableStateOf<Confirmacion?>(null)
        private set
    private var siguienteId = 0L

    fun mostrar(texto: String) {
        actual = Confirmacion(texto, ++siguienteId)
    }

    /// Sólo oculta si sigue a la vista la misma confirmación: una nueva que
    /// llegó mientras tanto no se borra con el tiempo de la anterior.
    fun ocultar(id: Long) {
        if (actual?.id == id) actual = null
    }
}

/// Franja verde que sube desde abajo cuando un registro sale bien y se va
/// sola a los [MS_CONFIRMACION_VISIBLE]. Antes la única señal era volver a la
/// lista (o un texto chico arriba), fácil de pasar por alto con el teléfono
/// en una mano y una cédula en la otra.
@Composable
fun FranjaConfirmacion(confirmacion: ConfirmacionReciente, modifier: Modifier = Modifier) {
    val actual = confirmacion.actual
    // La última que se vio, para seguir dibujándola mientras se va.
    var mostrada by remember { mutableStateOf<Confirmacion?>(null) }
    if (actual != null) mostrada = actual
    LaunchedEffect(actual?.id) {
        val id = actual?.id ?: return@LaunchedEffect
        delay(MS_CONFIRMACION_VISIBLE)
        confirmacion.ocultar(id)
    }
    AnimatedVisibility(
        visible = actual != null,
        modifier = modifier,
        enter = fadeIn(tween(MS_FUNDIDO_ENTRADA)) + slideInVertically(tween(MS_ENTRADA)) { alto -> alto },
        exit = fadeOut(tween(MS_SALIDA)) + slideOutVertically(tween(MS_SALIDA)) { alto -> alto },
        label = "franjaConfirmacion",
    ) {
        Surface(
            shape = RoundedCornerShape(12.dp),
            color = ColorExitoBrisas,
            // El verde de Tokyo Night es claro: ahí el texto va oscuro.
            contentColor = if (temaActual().oscuro) Color(0xFF16161E) else Color.White,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Row(modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Default.CheckCircle, contentDescription = null, modifier = Modifier.size(22.dp))
                Spacer(Modifier.width(12.dp))
                Text(
                    mostrada?.texto.orEmpty(),
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

internal const val MS_CONFIRMACION_VISIBLE = 2_500L
