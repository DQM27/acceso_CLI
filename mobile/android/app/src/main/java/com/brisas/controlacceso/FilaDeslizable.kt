package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.Text
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

/// Una fila que se desliza hacia la izquierda para registrar la salida (ver
/// `ActivosViewModel.deslizarSalida`: se puede deshacer unos segundos). Tocar
/// la fila sigue abriendo la confirmación de siempre; esto es un atajo, no un
/// reemplazo.
///
/// Sólo hacia la izquierda: hacia la derecha no pasa nada, para que un roce
/// al desplazar la lista no registre nada.
///
/// El fondo con el color de la sección y el texto "Salida" aparece sólo
/// mientras se arrastra (en reposo la fila queda como siempre), y al
/// completar el gesto vibra con el mismo toque corto que la confirmación
/// del escáner.
@Composable
internal fun FilaDeslizable(
    onSalida: () -> Unit,
    modifier: Modifier = Modifier,
    contenido: @Composable () -> Unit,
) {
    val contexto = LocalContext.current
    val salidaActual by rememberUpdatedState(onSalida)
    val estado = rememberSwipeToDismissBoxState(
        confirmValueChange = { valor ->
            if (valor == SwipeToDismissBoxValue.EndToStart) {
                vibrarConfirmacion(contexto)
                salidaActual()
                true
            } else {
                false
            }
        },
    )
    SwipeToDismissBox(
        state = estado,
        modifier = modifier,
        enableDismissFromStartToEnd = false,
        backgroundContent = {
            // Sólo mientras se arrastra: en reposo no se pinta nada detrás.
            if (estado.dismissDirection == SwipeToDismissBoxValue.EndToStart) {
                Row(
                    modifier = Modifier
                        .fillMaxSize()
                        .clip(MaterialTheme.shapes.medium)
                        .background(MaterialTheme.colorScheme.primary)
                        .padding(horizontal = 20.dp),
                    horizontalArrangement = Arrangement.End,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        "Salida",
                        color = MaterialTheme.colorScheme.onPrimary,
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier.padding(end = 8.dp),
                    )
                    Icon(
                        Icons.AutoMirrored.Filled.Logout,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onPrimary,
                        modifier = Modifier.size(22.dp),
                    )
                }
            } else {
                Row(modifier = Modifier.fillMaxSize().background(Color.Transparent)) {}
            }
        },
    ) {
        contenido()
    }
}
