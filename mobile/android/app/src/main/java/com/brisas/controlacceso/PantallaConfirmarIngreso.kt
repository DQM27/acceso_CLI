package com.brisas.controlacceso

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Icon
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.OutlinedTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.MedioIngreso
import uniffi.control_acceso_mobile.PreparacionIngreso

/// Formulario de ingreso de un contratista ya preparado. El registro (la
/// validación de gafete/placa y la llamada al núcleo) vive en
/// [ActivosViewModel.registrarIngreso] -- punto M2 de la auditoría móvil;
/// antes esta pantalla llamaba al núcleo directo y manejaba sus propias
/// corrutinas y errores. Acá sólo quedan los campos del formulario:
/// `medio`/`gafeteTexto`/`placaTexto` en `rememberSaveable` (sobreviven a
/// una rotación) y se pierden con la pantalla, así cada contratista
/// arranca con un formulario fresco.
@Composable
fun PantallaConfirmarIngreso(
    preparacion: PreparacionIngreso,
    error: String?,
    registrando: Boolean,
    onRegistrar: (medio: MedioIngreso, gafeteTexto: String, placaTexto: String) -> Unit,
    onLimpiarError: () -> Unit,
    onCambiar: () -> Unit,
) {
    RegistrarPantalla("confirmar_ingreso")
    var medio by rememberSaveable { mutableStateOf(MedioIngreso.CAMINANDO) }
    var gafeteTexto by rememberSaveable { mutableStateOf("") }
    var placaTexto by rememberSaveable { mutableStateOf("") }
    var escanerGafeteAbierto by remember { mutableStateOf(false) }
    BackHandler(enabled = !escanerGafeteAbierto, onBack = onCambiar)
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

    if (escanerGafeteAbierto) {
        PantallaEscanearCedula(
            modo = ModoEscaneoDocumento.GAFETE_CONTRATISTA,
            onDocumentoDetectado = { documento ->
                val limpio = documento.numeroDocumento.filter(Char::isDigit)
                gafeteTexto = limpio
                onLimpiarError()
                escanerGafeteAbierto = false
                // Escanear ya es una acción deliberada -- a diferencia de
                // tipear a mano, no hace falta un check "Automático" aparte
                // para decidir si dispara el ingreso solo (pedido explícito
                // del usuario 2026-09-20: quitar esa lógica y ese espacio en
                // pantalla). Tipeado a mano sigue requiriendo el botón.
                val gafete = limpio.toLongOrNull()
                // Con Vehículo, sólo dispara el ingreso solo si la placa ya
                // se había completado antes de escanear -- de lo contrario
                // el operador todavía tiene que llenarla, mismo criterio que
                // "Tipeado a mano sigue requiriendo el botón" arriba.
                val placaLista = medio == MedioIngreso.CAMINANDO || placaTexto.isNotBlank()
                if (gafete != null && placaLista) {
                    onRegistrar(medio, limpio, placaTexto)
                }
            },
            onCerrar = { escanerGafeteAbierto = false },
        )
        return
    }

    // Mismo criterio que [PantallaRutas]/[PantallaProveedores]/
    // [PantallaNuevoContratista]: `imePadding()` va en un Column SIN
    // `fillMaxSize` -- combinarlo con `fillMaxSize` deja un hueco enorme
    // entre el teclado y el contenido (bug reportado 2026-09-20).
    val scrollStateFormulario = rememberScrollState()
    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
    Column(
        modifier = Modifier.verticalScroll(scrollStateFormulario).imePadding(),
    ) {
        // Mismo lenguaje que [PantallaProveedores]: "← Volver" arriba a la
        // derecha en vez de un botón propio abajo de todo (pedido explícito
        // del usuario 2026-09-20, para homogeneizar los dos formularios).
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(
                preparacion.nombre,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.weight(1f),
            )
            BotonDiscretoBrisas(onClick = onCambiar) {
                Text("← Volver")
            }
        }
        Text(
            "${preparacion.cedula} · ${preparacion.empresaNombre}",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(bottom = 20.dp),
        )

        // Texto y cuenta de días del núcleo (`aviso_praind`, con su reloj).
        preparacion.avisoPraind?.let { aviso ->
            Text(
                "⚠ $aviso",
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
                // Descarta la placa tipeada antes si el operador vuelve a
                // Caminando: el núcleo descarta la placa (`placa_segun_medio`).
                val elegir = {
                    medio = opcion
                    if (opcion == MedioIngreso.CAMINANDO) placaTexto = ""
                }
                Row(
                    modifier = Modifier
                        .selectable(selected = medio == opcion, onClick = elegir)
                        .padding(end = 16.dp),
                ) {
                    RadioButton(selected = medio == opcion, onClick = elegir)
                    Text(etiqueta, modifier = Modifier.padding(top = 12.dp, start = 4.dp))
                }
            }
        }

        if (medio == MedioIngreso.VEHICULO) {
            OutlinedTextField(
                value = placaTexto,
                onValueChange = { placaTexto = it },
                label = { Text("Placa del vehículo") },
                singleLine = true,
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().padding(bottom = 16.dp),
            )
        }

        if (preparacion.requiereGafete) {
            Row(
                modifier = Modifier.fillMaxWidth().padding(bottom = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedTextField(
                    value = gafeteTexto,
                    onValueChange = { gafeteTexto = it.filter(Char::isDigit) },
                    label = { Text("Número de gafete") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    shape = FormaCampoBrisas,
                    colors = ColoresCampoBrisas(),
                    // Al enfocar este último input, lleva el scroll hasta el
                    // fondo -- ahí vive "Registrar entrada", último elemento
                    // de esta misma Column. El foco por sí solo sólo
                    // garantiza que el campo entre en pantalla, no el botón
                    // de abajo (pedido explícito del usuario 2026-09-20). Un
                    // solo `animateScrollTo` no alcanza: el teclado tarda
                    // ~250ms en animarse y el `imePadding()` va agrandando
                    // la Column cuadro a cuadro, así que `maxValue` todavía
                    // no refleja el alto final en el instante del foco -- se
                    // repite mientras dura esa animación.
                    modifier = Modifier.weight(1f).focusRequester(focoGafete)
                        .onFocusChanged {
                            if (it.isFocused) {
                                alcance.launch {
                                    repeat(15) {
                                        scrollStateFormulario.animateScrollTo(scrollStateFormulario.maxValue)
                                        delay(30)
                                    }
                                }
                            }
                        },
                )
                BotonDiscretoBrisas(
                    onClick = { escanerGafeteAbierto = true },
                    modifier = Modifier.padding(start = 8.dp),
                ) {
                    Icon(
                        Icons.Default.PhotoCamera,
                        contentDescription = "Escanear gafete",
                        modifier = Modifier.size(32.dp),
                    )
                }
            }
        }

        val mensajeError = error
        if (mensajeError != null) {
            Text(mensajeError, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(bottom = 12.dp))
        }

        // Mismo criterio que [PantallaProveedores]/[PantallaRutas]: el botón
        // se ve gris (deshabilitado) mientras falten datos, en vez de dejar
        // que el error salga recién al tocarlo -- pedido explícito del
        // usuario 2026-09-20, para que las dos pantallas se sientan igual.
        // Cuando el contratista no requiere gafete no hay nada más que
        // completar (el medio de ingreso ya arranca con un valor elegido),
        // así que el botón queda habilitado de entrada.
        val gafeteListo = !preparacion.requiereGafete || gafeteTexto.trim().toLongOrNull() != null
        val placaLista = medio != MedioIngreso.VEHICULO || placaTexto.isNotBlank()
        BotonBrisas(
            onClick = { onRegistrar(medio, gafeteTexto, placaTexto) },
            enabled = !registrando && gafeteListo && placaLista,
            modifier = Modifier.fillMaxWidth().focusRequester(focoConfirmar),
        ) {
            Text(if (registrando) "Registrando…" else "Registrar ingreso")
        }
    }
    }
}

@Composable
fun PantallaIngresoBloqueado(preparacion: PreparacionIngreso, mensaje: String, onCambiar: () -> Unit) {
    BackHandler(onBack = onCambiar)
    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        // Mismo lenguaje que [PantallaProveedores]/[PantallaConfirmarIngreso]:
        // "← Volver" arriba a la derecha en vez de un botón propio abajo de
        // todo (pedido explícito del usuario 2026-09-20, para que las tres
        // pantallas se vean igual).
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(
                preparacion.nombre,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.weight(1f),
            )
            BotonDiscretoBrisas(onClick = onCambiar) {
                Text("← Volver")
            }
        }
        Text(
            "${preparacion.cedula} · ${preparacion.empresaNombre}",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(bottom = 20.dp),
        )
        // Negrita -- pedido explícito del usuario 2026-09-20: que el motivo
        // de la denegación se lea con más fuerza que el resto del texto.
        Text(mensaje, color = MaterialTheme.colorScheme.error, fontWeight = FontWeight.Bold)
    }
}
