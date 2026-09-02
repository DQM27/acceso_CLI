package com.brisas.controlacceso

// NOTA DE ARQUITECTURA — leer mobile/app/ARQUITECTURA.md antes de tocar
// este archivo. Mismo patrón MVP a corregir: estado de formulario y
// llamada a `Nucleo.registrarIngreso` viven en el Composable. Al tocarlo,
// mover ese estado a un ViewModel propio (o compartirlo con el de
// `PantallaActivos.kt`, que es quien lo invoca) en vez de agregarle más
// campos o validaciones acá directamente.

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.MedioIngreso
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.PreparacionIngreso
import uniffi.control_acceso_mobile.ResultadoAcceso

/// Misma decisión que `desktop/src/api/ingresos.ts` (puedeContinuar /
/// mensajeBloqueo): `preparar_ingreso` no rechaza estos casos, ya vienen
/// calculados por Rust (`verificar_acceso`) — esto solo lee el resultado.
fun puedeContinuar(preparacion: PreparacionIngreso): Boolean =
    !preparacion.tieneIngresoActivo && preparacion.resultadoAcceso !is ResultadoAcceso.Denegado

fun mensajeBloqueo(preparacion: PreparacionIngreso): String {
    if (preparacion.tieneIngresoActivo) {
        return "El contratista ya tiene un ingreso activo."
    }
    val resultado = preparacion.resultadoAcceso
    if (resultado is ResultadoAcceso.Denegado) {
        return mensajeMotivoDenegacion(resultado.motivo)
    }
    return "No se puede continuar con este contratista."
}

fun mensajeMotivoDenegacion(motivo: uniffi.control_acceso_mobile.MotivoDenegacion): String =
    when (motivo) {
        uniffi.control_acceso_mobile.MotivoDenegacion.SIN_ACCESO -> "Acceso denegado · no tiene acceso autorizado"
        uniffi.control_acceso_mobile.MotivoDenegacion.PRAIND_VENCIDO -> "Acceso denegado · PRAIND vencido"
        uniffi.control_acceso_mobile.MotivoDenegacion.PRAIND_NO_REGISTRADO ->
            "Acceso denegado · PRAIND sin fecha registrada"
        uniffi.control_acceso_mobile.MotivoDenegacion.EMPRESA_INACTIVA -> "Acceso denegado · la empresa está inactiva"
    }

@Composable
fun PantallaConfirmarIngreso(
    nucleo: Nucleo,
    preparacion: PreparacionIngreso,
    onRegistrado: () -> Unit,
    onCambiar: () -> Unit,
) {
    var medio by remember { mutableStateOf(MedioIngreso.CAMINANDO) }
    var gafeteTexto by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var enviando by remember { mutableStateOf(false) }
    val alcance = rememberCoroutineScope()

    val focoGafete = remember { FocusRequester() }
    val focoConfirmar = remember { FocusRequester() }

    // Mismo criterio que NuevoIngresoModal.tsx: sin gafete que requiera el
    // foco, éste se va directo al botón — Enter sobre un botón también
    // confirma, así que no se pierde el atajo de teclado.
    LaunchedEffect(preparacion) {
        if (preparacion.requiereGafete) {
            focoGafete.requestFocus()
        } else {
            focoConfirmar.requestFocus()
        }
    }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Text(preparacion.nombre, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
        Text(
            "${preparacion.cedula} · ${preparacion.empresaNombre}",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(bottom = 20.dp),
        )

        if (preparacion.resultadoAcceso == ResultadoAcceso.PermitidoConAdvertencia) {
            Text(
                "⚠ PRAIND próximo a vencer",
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(bottom = 12.dp),
            )
        }
        if (preparacion.gafetesDeuda.isNotEmpty()) {
            Text(
                "⚠ Este contratista debe el gafete " +
                    preparacion.gafetesDeuda.joinToString(", ") { "#${it.toString().padStart(2, '0')}" },
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(bottom = 12.dp),
            )
        }

        Text("Medio de ingreso", style = MaterialTheme.typography.bodyMedium)
        Row(modifier = Modifier.padding(bottom = 16.dp)) {
            listOf(MedioIngreso.CAMINANDO to "Caminando", MedioIngreso.VEHICULO to "Vehículo").forEach { (opcion, etiqueta) ->
                Row(
                    modifier = Modifier
                        .selectable(selected = medio == opcion, onClick = { medio = opcion })
                        .padding(end = 16.dp),
                ) {
                    RadioButton(selected = medio == opcion, onClick = { medio = opcion })
                    Text(etiqueta, modifier = Modifier.padding(top = 12.dp, start = 4.dp))
                }
            }
        }

        if (preparacion.requiereGafete) {
            OutlinedTextField(
                value = gafeteTexto,
                onValueChange = { gafeteTexto = it.filter(Char::isDigit) },
                label = { Text("Número de gafete") },
                singleLine = true,
                keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(keyboardType = KeyboardType.Number),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = MaterialTheme.colorScheme.primary,
                    focusedLabelColor = MaterialTheme.colorScheme.primary,
                ),
                modifier = Modifier.fillMaxWidth().focusRequester(focoGafete).padding(bottom = 16.dp),
            )
        }

        val mensajeError = error
        if (mensajeError != null) {
            Text(mensajeError, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(bottom = 12.dp))
        }

        Button(
            onClick = {
                error = null
                val gafete: Long? = if (preparacion.requiereGafete) {
                    val numero = gafeteTexto.trim().toLongOrNull()
                    if (numero == null) {
                        error = if (gafeteTexto.isBlank()) "El gafete es requerido" else "Ingrese un número de gafete válido"
                        return@Button
                    }
                    numero
                } else {
                    null
                }
                enviando = true
                alcance.launch {
                    try {
                        withContext(Dispatchers.Default) {
                            nucleo.registrarIngreso(preparacion.contratistaId, medio, gafete)
                        }
                        onRegistrado()
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
            modifier = Modifier.fillMaxWidth().focusRequester(focoConfirmar),
        ) {
            Text(if (enviando) "Registrando…" else "Registrar entrada")
        }

        TextButton(onClick = onCambiar, modifier = Modifier.padding(top = 8.dp)) {
            Text("← Cambiar contratista")
        }
    }
}

@Composable
fun PantallaIngresoBloqueado(preparacion: PreparacionIngreso, mensaje: String, onCambiar: () -> Unit) {
    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Text(preparacion.nombre, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
        Text(
            "${preparacion.cedula} · ${preparacion.empresaNombre}",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(bottom = 20.dp),
        )
        Text(mensaje, color = MaterialTheme.colorScheme.error)
        OutlinedButton(onClick = onCambiar, modifier = Modifier.padding(top = 16.dp)) {
            Text("← Cambiar contratista")
        }
    }
}
