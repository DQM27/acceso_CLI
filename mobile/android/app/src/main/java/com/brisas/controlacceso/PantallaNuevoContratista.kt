package com.brisas.controlacceso

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
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
///
/// Recuperada 2026-09-12 tras haberse sacado el 2026-09-06 (ver
/// ARQUITECTURA.md) -- ahora con una segunda vía de captura además del
/// tipeo manual: escanear el carnet PRAIND del contratista con la misma
/// cámara/OCR que ya usa `PantallaEscanearCedula` para confirmar ingresos.
/// El carnet PRAIND es el único tipo de documento que trae, además de
/// cédula y nombre, la empresa y el vencimiento de la inducción -- por eso
/// es el que de verdad ahorra tipeo acá (ver `LectorDocumentosIdentidad.extraerPraind`).
private fun requierePraind(tipo: TipoIngreso, personalRuta: Boolean): Boolean =
    personalRuta || tipo == TipoIngreso.PRAIND || tipo == TipoIngreso.IN_HOUSE

private fun etiquetaTipo(tipo: TipoIngreso): String =
    when (tipo) {
        TipoIngreso.PRAIND -> "PRAIND"
        TipoIngreso.IN_HOUSE -> "In-house"
        TipoIngreso.POR_CORREO -> "Por correo"
        TipoIngreso.SWAT -> "SWAT"
    }

/// Misma convención día-mes-año que el resto de la app (`Tiempo.kt`,
/// `desktop/src/tiempo.ts`) -- Rust exige ISO (`AAAA-MM-DD`) para parsear la
/// fecha, pero mostrarla así en el formulario rompía la convención que la
/// persona ya espera en cualquier otra pantalla.
private fun FechaDocumento.aTextoDDMMYYYY(): String = "%02d-%02d-%04d".format(dia, mes, anio)

/// Inverso de [aTextoDDMMYYYY] -- convierte lo que la persona tipeó
/// (día-mes-año) al formato ISO que espera `DatosContratista.fechaVencimientoPraind`
/// en Rust. Si el texto no tiene la forma esperada se devuelve tal cual: Rust
/// igual la rechaza con un error legible que cita el texto original.
private fun textoDDMMYYYYaIso(texto: String): String {
    val partes = texto.split("-")
    if (partes.size != 3) return texto
    val (dia, mes, anio) = partes
    return "%s-%s-%s".format(anio.padStart(4, '0'), mes.padStart(2, '0'), dia.padStart(2, '0'))
}

/// Compara texto libre del carnet contra los nombres reales de
/// `Nucleo.listarEmpresas()` -- ninguna de las dos fuentes garantiza
/// mayúsculas/espacios idénticos, así que primero se intenta una igualdad
/// exacta (sin distinguir mayúsculas) y sólo si eso falla se cae a
/// "una contiene a la otra" (tolera un "S.A." de más o de menos a un lado).
private fun buscarEmpresaPorNombre(empresas: List<Empresa>, textoCarnet: String): Empresa? {
    val normalizado = textoCarnet.trim()
    if (normalizado.isEmpty()) return null
    return empresas.firstOrNull { it.nombre.equals(normalizado, ignoreCase = true) }
        ?: empresas.firstOrNull {
            it.nombre.contains(normalizado, ignoreCase = true) ||
                normalizado.contains(it.nombre, ignoreCase = true)
        }
}

/// Sin ViewModel a propósito — mismo motivo que `PantallaConfirmarIngreso`
/// (ver su doc-comment y ARQUITECTURA.md): `PantallaPrincipal` desmonta
/// este formulario por completo al volver, así que cada entrada ya es un
/// intento fresco sin necesidad de un dueño de estado que sobreviva más
/// que eso.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PantallaNuevoContratista(nucleo: Nucleo, onVolver: () -> Unit) {
    var empresas by remember { mutableStateOf<List<Empresa>>(emptyList()) }
    var cedula by rememberSaveable { mutableStateOf("") }
    var nombre by rememberSaveable { mutableStateOf("") }
    // `Empresa` es un `data class` generado por uniffi sin `Serializable` —
    // `rememberSaveable` fallaría en tiempo de ejecución acá. Se pierde en
    // una rotación (a diferencia del resto del formulario); no vale la
    // complejidad de un `Saver` a mano sólo para este campo.
    var empresaSeleccionada by remember { mutableStateOf<Empresa?>(null) }
    // Nombre de empresa leído del carnet cuando no calzó con ninguna fila
    // de `empresas` -- se muestra como pista para elegir a mano, nunca se
    // manda a Rust (que sólo acepta un `empresa_id` real).
    var empresaSugeridaTexto by remember { mutableStateOf<String?>(null) }
    var tipoIngreso by rememberSaveable { mutableStateOf(TipoIngreso.PRAIND) }
    var personalRuta by rememberSaveable { mutableStateOf(false) }
    var tieneAcceso by rememberSaveable { mutableStateOf(true) }
    var fechaPraind by rememberSaveable { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var mensaje by remember { mutableStateOf<String?>(null) }
    var enviando by remember { mutableStateOf(false) }
    var menuEmpresaAbierto by remember { mutableStateOf(false) }
    var menuTipoAbierto by remember { mutableStateOf(false) }
    var escaneando by remember { mutableStateOf(false) }
    val alcance = rememberCoroutineScope()

    LaunchedEffect(Unit) {
        try {
            empresas = withContext(Dispatchers.Default) { nucleo.listarEmpresas() }
        } catch (excepcion: NucleoException) {
            error = excepcion.message
        }
    }

    if (escaneando) {
        PantallaEscanearCedula(
            modo = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
            onDocumentoDetectado = { documento ->
                escaneando = false
                error = null
                if (documento.numeroDocumento.isNotBlank()) {
                    cedula = documento.numeroDocumento.filter(Char::isDigit).ifBlank { documento.numeroDocumento }
                }
                documento.nombre?.let { nombre = it }
                if (documento.tipo == TipoDocumento.CARNET_INDUCCION_PRAIND) {
                    tipoIngreso = TipoIngreso.PRAIND
                    documento.vencimiento?.let { fechaPraind = it.aTextoDDMMYYYY() }
                    val textoEmpresa = documento.empresa?.trim()
                    when {
                        textoEmpresa.isNullOrBlank() -> Unit
                        else -> {
                            val coincidencia = buscarEmpresaPorNombre(empresas, textoEmpresa)
                            if (coincidencia != null) {
                                empresaSeleccionada = coincidencia
                                empresaSugeridaTexto = null
                            } else {
                                empresaSugeridaTexto = textoEmpresa
                            }
                        }
                    }
                    mensaje = "Carnet PRAIND leído — revise los datos antes de guardar"
                } else {
                    mensaje = "Se detectó ${documento.tipo.nombreLegible()}, no un carnet PRAIND — " +
                        "complete empresa y vencimiento a mano"
                }
            },
            onCerrar = { escaneando = false },
        )
        return
    }

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text("Nuevo contratista", style = MaterialTheme.typography.titleMedium)
            BotonDiscretoBrisas(onClick = onVolver) {
                Text("← Volver")
            }
        }

        BotonBrisas(
            onClick = { escaneando = true },
            modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
        ) {
            Icon(Icons.Default.PhotoCamera, contentDescription = null, modifier = Modifier.padding(end = 8.dp))
            Text("Escanear carnet PRAIND")
        }

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
            modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
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
                            empresaSugeridaTexto = null
                            menuEmpresaAbierto = false
                        },
                    )
                }
            }
        }
        val sugerenciaEmpresa = empresaSugeridaTexto
        if (sugerenciaEmpresa != null) {
            Text(
                "El carnet dice \"$sugerenciaEmpresa\" — no hay ninguna empresa igual en la lista, elija la correcta arriba.",
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.fillMaxWidth().padding(top = 4.dp),
            )
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

        if (requierePraind(tipoIngreso, personalRuta)) {
            OutlinedTextField(
                value = fechaPraind,
                onValueChange = { fechaPraind = it.filter { c -> c.isDigit() || c == '-' } },
                label = { Text("Vencimiento PRAIND (DD-MM-AAAA)") },
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
                                    fechaVencimientoPraind = fechaPraind.ifBlank { null }?.let(::textoDDMMYYYYaIso),
                                    esPersonalRuta = personalRuta,
                                    tieneAcceso = tieneAcceso,
                                ),
                            )
                        }
                        CambiosNube.solicitar()
                        mensaje = "Contratista registrado: $nombre"
                        cedula = ""
                        nombre = ""
                        empresaSeleccionada = null
                        empresaSugeridaTexto = null
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
