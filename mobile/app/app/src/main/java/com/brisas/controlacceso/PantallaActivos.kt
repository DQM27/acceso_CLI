package com.brisas.controlacceso

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.IngresoActivoResumen
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.ResultadoAcceso

/// A diferencia de desktop (SalidaModal.tsx), que agrega un modo "por
/// gafete" con texto separado por comas para aprovechar el teclado del
/// guardia en la PC, aquí no hay atajos de teclado que aprovechar — todo es
/// táctil. Se simplifica a lo directo: tocar la fila, confirmar en un
/// diálogo, listo.
@Composable
fun PantallaActivos(nucleo: Nucleo) {
    var texto by remember { mutableStateOf("") }
    var activos by remember { mutableStateOf<List<IngresoActivoResumen>>(emptyList()) }
    var error by remember { mutableStateOf<String?>(null) }
    var seleccionado by remember { mutableStateOf<IngresoActivoResumen?>(null) }
    var recargas by remember { mutableIntStateOf(0) }
    val alcance = rememberCoroutineScope()

    LaunchedEffect(texto, recargas) {
        try {
            activos = withContext(Dispatchers.Default) { nucleo.listarIngresosActivos(texto) }
            error = null
        } catch (excepcion: Exception) {
            error = excepcion.message
        }
    }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Text(
            "Ingresos activos",
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.padding(bottom = 12.dp),
        )

        OutlinedTextField(
            value = texto,
            onValueChange = { texto = it },
            label = { Text("Cédula, nombre o empresa") },
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
            items(activos, key = { it.registroId }) { activo ->
                FilaActivo(activo, onClick = { seleccionado = activo })
                HorizontalDivider(color = MaterialTheme.colorScheme.outline)
            }
        }
    }

    val activo = seleccionado
    if (activo != null) {
        AlertDialog(
            onDismissRequest = { seleccionado = null },
            title = { Text("Registrar salida") },
            text = {
                Text("${activo.contratistaNombre} · ${activo.cedula} · ${activo.empresaNombre}")
            },
            confirmButton = {
                TextButton(onClick = {
                    seleccionado = null
                    alcance.launch {
                        try {
                            withContext(Dispatchers.Default) { nucleo.registrarSalida(activo.registroId) }
                            recargas++
                        } catch (excepcion: Exception) {
                            error = excepcion.message
                        }
                    }
                }) {
                    Text("Confirmar")
                }
            },
            dismissButton = {
                TextButton(onClick = { seleccionado = null }) {
                    Text("Cancelar")
                }
            },
        )
    }
}

@Composable
private fun FilaActivo(activo: IngresoActivoResumen, onClick: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick).padding(vertical = 10.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(activo.contratistaNombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Text(
            "${activo.cedula} · ${activo.empresaNombre}" +
                if (activo.gafeteNumero != null) " · Gafete ${activo.gafeteNumero}" else " · Sin gafete",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            textoEstadoAcceso(activo.resultadoAcceso),
            style = MaterialTheme.typography.bodySmall,
            color = colorEstadoAcceso(activo.resultadoAcceso),
        )
    }
}

private fun textoEstadoAcceso(resultado: ResultadoAcceso): String =
    when (resultado) {
        is ResultadoAcceso.Permitido -> "Al día"
        is ResultadoAcceso.PermitidoConAdvertencia -> "PRAIND próximo a vencer"
        is ResultadoAcceso.Denegado -> mensajeMotivoDenegacion(resultado.motivo)
    }

@Composable
private fun colorEstadoAcceso(resultado: ResultadoAcceso) =
    when (resultado) {
        is ResultadoAcceso.Permitido -> MaterialTheme.colorScheme.primary
        is ResultadoAcceso.PermitidoConAdvertencia -> MaterialTheme.colorScheme.error
        is ResultadoAcceso.Denegado -> MaterialTheme.colorScheme.error
    }
