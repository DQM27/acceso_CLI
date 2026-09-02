package com.brisas.controlacceso

// NOTA DE ARQUITECTURA — leer mobile/app/ARQUITECTURA.md antes de tocar
// este archivo. Es la pantalla más chica y la más simple (solo lectura),
// pero igual llama a `Nucleo.buscarHistorial` y guarda su resultado con
// `remember` en vez de un ViewModel. Al agregarle algo (paginación, filtro
// por fecha, exportar), extraer el `HistorialViewModel` en ese momento en
// vez de seguir creciendo el Composable.

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.MovimientoHistorial
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.ResultadoIngresoRegistrado

// Sólo lectura, sin acción — a diferencia de Buscar/Activos no hay nada que
// confirmar acá. Últimos 6 meses por defecto (mismo criterio que
// desktop/src/pantallas/Historial.tsx): registro_ingresos crece sin límite,
// nunca se trae "todo".
@Composable
fun PantallaHistorial(nucleo: Nucleo) {
    var texto by remember { mutableStateOf("") }
    var movimientos by remember { mutableStateOf<List<MovimientoHistorial>>(emptyList()) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(texto) {
        try {
            movimientos = withContext(Dispatchers.Default) { nucleo.buscarHistorial(texto) }
            error = null
        } catch (excepcion: Exception) {
            error = excepcion.message
        }
    }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        OutlinedTextField(
            value = texto,
            onValueChange = { texto = it },
            label = { Text("Cédula o nombre") },
            singleLine = true,
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth(),
        )

        val mensajeError = error
        if (mensajeError != null) {
            Text(
                mensajeError,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(top = 12.dp),
            )
        }

        LazyColumn(modifier = Modifier.padding(top = 12.dp)) {
            items(movimientos, key = { it.registroId }) { movimiento ->
                FilaMovimiento(movimiento)
                HorizontalDivider(color = MaterialTheme.colorScheme.outline)
            }
        }
    }
}

@Composable
private fun FilaMovimiento(movimiento: MovimientoHistorial) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(vertical = 10.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(movimiento.contratistaNombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Text(
            "${movimiento.cedula} · ${movimiento.empresaNombre}" +
                if (movimiento.gafeteNumero != null) " · Gafete ${movimiento.gafeteNumero}" else "",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Entrada ${textoFechaHora(movimiento.fechaHoraIngreso)} (${movimiento.usuarioIngresoNombre})",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        val salida = movimiento.fechaHoraSalida
        if (salida != null) {
            Text(
                "Salida ${textoFechaHora(salida)}" +
                    (movimiento.usuarioSalidaNombre?.let { " ($it)" } ?: ""),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        } else {
            Text(
                "Sin salida registrada",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.primary,
            )
        }
        if (movimiento.resultadoAcceso is ResultadoIngresoRegistrado.PermitidoConAdvertencia) {
            Text(
                "⚠ PRAIND próximo a vencer al momento del ingreso",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        }
    }
}
