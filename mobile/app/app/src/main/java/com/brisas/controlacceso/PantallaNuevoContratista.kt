package com.brisas.controlacceso

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.DatosContratista
import uniffi.control_acceso_mobile.Empresa
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.TipoIngreso

/// Mismo formulario que desktop/src/pantallas/FormularioContratista.tsx —
/// sólo alta, no edición (ver docs/plan-app-movil.md). La validación real
/// vuelve a correr en Rust (ContratistaService::crear); lo de acá es sólo
/// feedback inmediato, igual que el esquema de zod del lado desktop.
private fun requierePraind(tipo: TipoIngreso, personalRuta: Boolean): Boolean =
    personalRuta || tipo == TipoIngreso.PRAIND || tipo == TipoIngreso.IN_HOUSE

private fun etiquetaTipo(tipo: TipoIngreso): String =
    when (tipo) {
        TipoIngreso.PRAIND -> "PRAIND"
        TipoIngreso.IN_HOUSE -> "In-house"
        TipoIngreso.POR_CORREO -> "Por correo"
        TipoIngreso.SWAT -> "SWAT"
    }

/// Sin ViewModel a propósito — mismo motivo que `PantallaConfirmarIngreso`
/// (ver su doc-comment y mobile/app/ARQUITECTURA.md): `PantallaPrincipal`
/// desmonta este formulario por completo al volver ("← Volver"), así que
/// cada entrada ya es un intento fresco sin necesidad de un dueño de
/// estado que sobreviva más que eso.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PantallaNuevoContratista(nucleo: Nucleo) {
    var empresas by remember { mutableStateOf<List<Empresa>>(emptyList()) }
    var cedula by rememberSaveable { mutableStateOf("") }
    var nombre by rememberSaveable { mutableStateOf("") }
    // `Empresa` es un `data class` generado por uniffi sin `Serializable` —
    // `rememberSaveable` fallaría en tiempo de ejecución acá. Se pierde en
    // una rotación (a diferencia del resto del formulario); no vale la
    // complejidad de un `Saver` a mano sólo para este campo.
    var empresaSeleccionada by remember { mutableStateOf<Empresa?>(null) }
    var tipoIngreso by rememberSaveable { mutableStateOf(TipoIngreso.PRAIND) }
    var personalRuta by rememberSaveable { mutableStateOf(false) }
    var tieneAcceso by rememberSaveable { mutableStateOf(true) }
    var fechaPraind by rememberSaveable { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var mensaje by remember { mutableStateOf<String?>(null) }
    var enviando by remember { mutableStateOf(false) }
    var menuEmpresaAbierto by remember { mutableStateOf(false) }
    var menuTipoAbierto by remember { mutableStateOf(false) }
    val alcance = rememberCoroutineScope()

    LaunchedEffect(Unit) {
        try {
            empresas = withContext(Dispatchers.Default) { nucleo.listarEmpresas() }
        } catch (excepcion: NucleoException) {
            error = excepcion.message
        }
    }

    val mostrarPraind = requierePraind(tipoIngreso, personalRuta)

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp),
    ) {
        Text("Nuevo contratista", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(bottom = 16.dp))

        OutlinedTextField(
            value = cedula,
            onValueChange = { cedula = it.filter(Char::isDigit) },
            label = { Text("Cédula") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
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

        ExposedDropdownMenuBox(
            expanded = menuEmpresaAbierto,
            onExpandedChange = { menuEmpresaAbierto = it },
            modifier = Modifier.padding(top = 12.dp),
        ) {
            OutlinedTextField(
                value = empresaSeleccionada?.nombre ?: "",
                onValueChange = {},
                readOnly = true,
                label = { Text("Empresa") },
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = menuEmpresaAbierto) },
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = MaterialTheme.colorScheme.primary,
                    focusedLabelColor = MaterialTheme.colorScheme.primary,
                ),
                modifier = Modifier.fillMaxWidth().menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
            )
            DropdownMenu(expanded = menuEmpresaAbierto, onDismissRequest = { menuEmpresaAbierto = false }) {
                empresas.forEach { empresa ->
                    DropdownMenuItem(
                        text = { Text(empresa.nombre) },
                        onClick = {
                            empresaSeleccionada = empresa
                            menuEmpresaAbierto = false
                        },
                    )
                }
            }
        }

        ExposedDropdownMenuBox(
            expanded = menuTipoAbierto,
            onExpandedChange = { menuTipoAbierto = it },
            modifier = Modifier.padding(top = 12.dp),
        ) {
            OutlinedTextField(
                value = etiquetaTipo(tipoIngreso),
                onValueChange = {},
                readOnly = true,
                label = { Text("Tipo de ingreso") },
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = menuTipoAbierto) },
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = MaterialTheme.colorScheme.primary,
                    focusedLabelColor = MaterialTheme.colorScheme.primary,
                ),
                modifier = Modifier.fillMaxWidth().menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
            )
            DropdownMenu(expanded = menuTipoAbierto, onDismissRequest = { menuTipoAbierto = false }) {
                TipoIngreso.entries.forEach { tipo ->
                    DropdownMenuItem(
                        text = { Text(etiquetaTipo(tipo)) },
                        onClick = {
                            tipoIngreso = tipo
                            menuTipoAbierto = false
                        },
                    )
                }
            }
        }

        Row(modifier = Modifier.padding(top = 8.dp)) {
            Checkbox(checked = personalRuta, onCheckedChange = { personalRuta = it })
            Text("Personal de ruta", modifier = Modifier.padding(top = 12.dp))
        }
        Row {
            Checkbox(checked = tieneAcceso, onCheckedChange = { tieneAcceso = it })
            Text("Con acceso", modifier = Modifier.padding(top = 12.dp))
        }

        if (mostrarPraind) {
            OutlinedTextField(
                value = fechaPraind,
                onValueChange = { fechaPraind = it.filter { c -> c.isDigit() || c == '-' } },
                label = { Text("Vencimiento PRAIND (AAAA-MM-DD)") },
                singleLine = true,
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = MaterialTheme.colorScheme.primary,
                    focusedLabelColor = MaterialTheme.colorScheme.primary,
                ),
                modifier = Modifier.fillMaxWidth().padding(top = 4.dp),
            )
        }

        val mensajeError = error
        if (mensajeError != null) {
            Text(mensajeError, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(top = 16.dp))
        }
        val mensajeExito = mensaje
        if (mensajeExito != null) {
            Text(mensajeExito, color = ColorExitoBrisas, modifier = Modifier.padding(top = 16.dp))
        }

        BotonBrisas(
            onClick = {
                error = null
                mensaje = null
                val empresa = empresaSeleccionada
                if (cedula.isBlank() || nombre.isBlank() || empresa == null) {
                    error = "Complete cédula, nombre y empresa"
                    return@BotonBrisas
                }
                enviando = true
                alcance.launch {
                    try {
                        withContext(Dispatchers.Default) {
                            nucleo.crearContratista(
                                DatosContratista(
                                    cedula = cedula,
                                    nombre = nombre,
                                    empresaId = empresa.id,
                                    tipoIngreso = tipoIngreso,
                                    fechaVencimientoPraind = fechaPraind.ifBlank { null },
                                    esPersonalRuta = personalRuta,
                                    tieneAcceso = tieneAcceso,
                                ),
                            )
                        }
                        CambiosNube.solicitar()
                        mensaje = "Contratista registrado: $nombre"
                        cedula = ""
                        nombre = ""
                        fechaPraind = ""
                        personalRuta = false
                        tieneAcceso = true
                    } catch (excepcion: NucleoException) {
                        error = excepcion.message
                    } finally {
                        enviando = false
                    }
                }
            },
            enabled = !enviando,
            modifier = Modifier.fillMaxWidth().padding(top = 20.dp, bottom = 32.dp),
        ) {
            Text(if (enviando) "Guardando…" else "Guardar")
        }
    }
}
