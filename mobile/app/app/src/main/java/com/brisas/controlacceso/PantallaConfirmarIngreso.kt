package com.brisas.controlacceso

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.runtime.saveable.rememberSaveable
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
import uniffi.control_acceso_mobile.MotivoDenegacion
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
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

fun mensajeMotivoDenegacion(motivo: MotivoDenegacion): String =
    when (motivo) {
        MotivoDenegacion.SIN_ACCESO -> "Acceso denegado · no tiene acceso autorizado"
        MotivoDenegacion.PRAIND_VENCIDO -> "Acceso denegado · PRAIND vencido"
        MotivoDenegacion.PRAIND_NO_REGISTRADO -> "Acceso denegado · PRAIND sin fecha registrada"
        MotivoDenegacion.EMPRESA_INACTIVA -> "Acceso denegado · la empresa está inactiva"
    }

/// A diferencia de `PantallaActivos`/`PantallaLogin`, esta pantalla se
/// queda con `remember`/`rememberSaveable` en vez de un `ViewModel` — a
/// propósito, ver mobile/app/ARQUITECTURA.md sobre cuándo uno hace falta y
/// cuándo no:
///
/// El árbol de estados `SeleccionIngreso` en `ActivosViewModel` desmonta
/// por completo esta pantalla al cancelar o al confirmar (vuelve a
/// `Ninguna`), así que cada vez que se entra acá es una tentativa nueva —
/// exactamente el estado "fresco" que ya da gratis `remember` al perderse
/// junto con la composición. Un `ViewModel`, en cambio, sobrevive aunque el
/// Composable se desmonte — con el alcance por defecto de esta app (sin
/// Navigation-Compose, todos los `viewModel()` comparten el mismo dueño:
/// la Activity) eso arrastraría el `error`/`gafeteTexto` de un intento
/// fallido con el contratista A a la pantalla del contratista B, salvo que
/// se le pase una key que distinga cada intento — complejidad real que acá
/// no compra nada, porque no hay ninguna razón de negocio para que este
/// formulario sobreviva más que la propia pantalla.
///
/// Lo que sí se corrige: `medio`/`gafeteTexto` pasan a `rememberSaveable`
/// (sobreviven una rotación de pantalla a medio llenar, algo que
/// `remember` no da) y el `catch` pasa de `Exception` genérico a
/// [NucleoException] específico — mismo criterio que en los ViewModel de
/// las otras pantallas.
@Composable
fun PantallaConfirmarIngreso(
    nucleo: Nucleo,
    preparacion: PreparacionIngreso,
    onRegistrado: () -> Unit,
    onCambiar: () -> Unit,
) {
    var medio by rememberSaveable { mutableStateOf(MedioIngreso.CAMINANDO) }
    var gafeteTexto by rememberSaveable { mutableStateOf("") }
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
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
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
                    } catch (excepcion: NucleoException) {
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
