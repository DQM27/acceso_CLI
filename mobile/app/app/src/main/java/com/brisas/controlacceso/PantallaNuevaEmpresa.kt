package com.brisas.controlacceso

// NOTA DE ARQUITECTURA — leer mobile/app/ARQUITECTURA.md antes de tocar
// este archivo. Formulario simple, pero mismo patrón a corregir: estado y
// llamada a `Nucleo.crearEmpresa` en el Composable. Al agregarle campos o
// validaciones, extraer un `NuevaEmpresaViewModel` en vez de crecer acá.

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.Nucleo

@Composable
fun PantallaNuevaEmpresa(nucleo: Nucleo) {
    var nombre by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var mensaje by remember { mutableStateOf<String?>(null) }
    var enviando by remember { mutableStateOf(false) }
    val alcance = rememberCoroutineScope()

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Text("Nueva empresa", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(bottom = 16.dp))

        OutlinedTextField(
            value = nombre,
            onValueChange = { nombre = it },
            label = { Text("Nombre") },
            singleLine = true,
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth(),
        )

        val mensajeError = error
        if (mensajeError != null) {
            Text(mensajeError, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(top = 16.dp))
        }
        val mensajeExito = mensaje
        if (mensajeExito != null) {
            Text(mensajeExito, color = MaterialTheme.colorScheme.primary, modifier = Modifier.padding(top = 16.dp))
        }

        Button(
            onClick = {
                error = null
                mensaje = null
                if (nombre.isBlank()) {
                    error = "El nombre es obligatorio"
                    return@Button
                }
                enviando = true
                alcance.launch {
                    try {
                        withContext(Dispatchers.Default) { nucleo.crearEmpresa(nombre) }
                        mensaje = "Empresa registrada: $nombre"
                        nombre = ""
                    } catch (excepcion: Exception) {
                        error = excepcion.message
                    } finally {
                        enviando = false
                    }
                }
            },
            enabled = !enviando,
            colors = ButtonDefaults.buttonColors(
                containerColor = MaterialTheme.colorScheme.primary,
                contentColor = MaterialTheme.colorScheme.onPrimary,
            ),
            modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
        ) {
            Text(if (enviando) "Guardando…" else "Guardar")
        }
    }
}
