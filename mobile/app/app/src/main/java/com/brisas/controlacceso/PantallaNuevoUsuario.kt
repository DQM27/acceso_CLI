package com.brisas.controlacceso

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuDefaults
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
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.DatosUsuario
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.RolUsuario

private fun etiquetaRol(rol: RolUsuario): String =
    when (rol) {
        RolUsuario.ROOT -> "Root"
        RolUsuario.ADMINISTRADOR -> "Administrador"
        RolUsuario.OPERADOR -> "Operador"
    }

/// Sólo Root/Administrador llegan a esta pantalla (MainActivity oculta el
/// menú para Operador) — Rust vuelve a exigir lo mismo del lado real
/// (`Operacion::GestionarUsuarios`), así que no hay doble mantenimiento de
/// la regla, esto es sólo la UX de no ofrecer un botón que va a fallar.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PantallaNuevoUsuario(nucleo: Nucleo) {
    var cedula by remember { mutableStateOf("") }
    var nombre by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var rol by remember { mutableStateOf(RolUsuario.OPERADOR) }
    var activo by remember { mutableStateOf(true) }
    var menuRolAbierto by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var mensaje by remember { mutableStateOf<String?>(null) }
    var enviando by remember { mutableStateOf(false) }
    val alcance = rememberCoroutineScope()

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Text("Nuevo usuario", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(bottom = 16.dp))

        OutlinedTextField(
            value = cedula,
            onValueChange = { cedula = it.filter(Char::isDigit) },
            label = { Text("Cédula") },
            singleLine = true,
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth(),
        )

        OutlinedTextField(
            value = nombre,
            onValueChange = { nombre = it },
            label = { Text("Nombre") },
            singleLine = true,
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
        )

        OutlinedTextField(
            value = password,
            onValueChange = { password = it },
            label = { Text("Contraseña") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
        )

        ExposedDropdownMenuBox(
            expanded = menuRolAbierto,
            onExpandedChange = { menuRolAbierto = it },
            modifier = Modifier.padding(top = 12.dp),
        ) {
            OutlinedTextField(
                value = etiquetaRol(rol),
                onValueChange = {},
                readOnly = true,
                label = { Text("Rol") },
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = menuRolAbierto) },
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = MaterialTheme.colorScheme.primary,
                    focusedLabelColor = MaterialTheme.colorScheme.primary,
                ),
                modifier = Modifier.fillMaxWidth().menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
            )
            DropdownMenu(expanded = menuRolAbierto, onDismissRequest = { menuRolAbierto = false }) {
                RolUsuario.entries.forEach { opcion ->
                    DropdownMenuItem(
                        text = { Text(etiquetaRol(opcion)) },
                        onClick = {
                            rol = opcion
                            menuRolAbierto = false
                        },
                    )
                }
            }
        }

        Row(modifier = Modifier.padding(top = 8.dp)) {
            Checkbox(checked = activo, onCheckedChange = { activo = it })
            Text("Activo", modifier = Modifier.padding(top = 12.dp))
        }

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
                if (cedula.isBlank() || nombre.isBlank() || password.isBlank()) {
                    error = "Complete cédula, nombre y contraseña"
                    return@Button
                }
                enviando = true
                alcance.launch {
                    try {
                        withContext(Dispatchers.Default) {
                            nucleo.crearUsuario(
                                DatosUsuario(
                                    cedula = cedula,
                                    nombre = nombre,
                                    password = password,
                                    rol = rol,
                                    activo = activo,
                                ),
                            )
                        }
                        mensaje = "Usuario registrado: $nombre"
                        cedula = ""
                        nombre = ""
                        password = ""
                        rol = RolUsuario.OPERADOR
                        activo = true
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
