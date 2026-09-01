package com.brisas.controlacceso

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
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
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.TipoIngreso
import uniffi.control_acceso_mobile.UsuarioSesion

// Vía primaria del guardia (ver docs/plan-app-movil.md): busca directo sobre
// la copia local en SQLite, sin red de por medio, así que cada tecla llama
// de nuevo a Nucleo.buscarContratistas — la decisión de qué cuenta como
// coincidencia sigue siendo de Rust (BusquedaTexto), esta pantalla solo pinta.
@Composable
fun PantallaContratistas(nucleo: Nucleo, sesion: UsuarioSesion) {
    var texto by remember { mutableStateOf("") }
    var resultados by remember { mutableStateOf<List<ContratistaResumen>>(emptyList()) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(texto) {
        try {
            resultados = withContext(Dispatchers.Default) { nucleo.buscarContratistas(texto) }
            error = null
        } catch (excepcion: Exception) {
            error = excepcion.message
        }
    }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Text(
            "Hola, ${sesion.nombre}",
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.padding(bottom = 12.dp),
        )

        OutlinedTextField(
            value = texto,
            onValueChange = { texto = it },
            label = { Text("Buscar contratista") },
            leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
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
            items(resultados, key = { it.id }) { contratista ->
                FilaContratista(contratista)
                HorizontalDivider(color = MaterialTheme.colorScheme.outline)
            }
        }
    }
}

@Composable
private fun FilaContratista(contratista: ContratistaResumen) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(vertical = 10.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(
            contratista.nombre,
            style = MaterialTheme.typography.bodyLarge,
            fontWeight = FontWeight.Medium,
        )
        Text(
            "${contratista.cedula} · ${contratista.empresaNombre} · ${etiquetaTipoIngreso(contratista.tipoIngreso)}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        if (!contratista.tieneAcceso) {
            Text(
                "Sin acceso autorizado",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        }
        if (contratista.tieneIngresoActivo) {
            Text(
                "Ingreso activo (sin salida registrada)",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.primary,
            )
        }
    }
}

private fun etiquetaTipoIngreso(tipo: TipoIngreso): String =
    when (tipo) {
        TipoIngreso.PRAIND -> "PRAIND"
        TipoIngreso.IN_HOUSE -> "In-house"
        TipoIngreso.POR_CORREO -> "Por correo"
        TipoIngreso.SWAT -> "SWAT"
    }
