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
import java.time.LocalDate
import java.time.temporal.ChronoUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.MedioIngreso
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.PreparacionIngreso
import uniffi.control_acceso_mobile.ResultadoAcceso

/// El gafete ya está activo en este sitio del lado de otro dispositivo --
/// usada por [GafetesProvisionalesViewModel]/[ProveedoresViewModel], que
/// todavía hacen el chequeo y la escritura como dos pasos separados. Esta
/// misma pantalla ([PantallaConfirmarIngreso]) ya NO la usa -- desde que
/// `Nucleo.registrarIngresoConSecreto` fusiona el chequeo con la
/// escritura, el mismo caso llega acá como [NucleoException]
/// (`NucleoError::GafeteOcupadoEnSitio` en Rust), con el mismo texto.
class GafeteOcupadoEnSitioException(numero: Long) :
    Exception("El gafete $numero ya está en uso en otro dispositivo de la unidad operativa")

/// Espejo de `mensajeVencimientoPraind` (`desktop/src/api/ingresos.ts`) --
/// antes esta pantalla sólo mostraba "PRAIND próximo a vencer" sin decir
/// cuánto quedaba, mientras desktop ya avisaba "vence en N días (fecha)".
/// `fecha` en ISO (`AAAA-MM-DD`), igual que la manda `PreparacionIngreso`,
/// pero se muestra día-mes-año -- misma convención que el resto de la app
/// (`FechaDocumento.aTextoDDMMYYYY`) -- mostrar el ISO crudo entre
/// paréntesis era inconsistente con eso (hallazgo 2026-09-20).
fun mensajeVencimientoPraind(fecha: String): String {
    val fechaParseada = LocalDate.parse(fecha)
    val dias = ChronoUnit.DAYS.between(LocalDate.now(), fechaParseada)
    val cuenta = when {
        dias <= 0 -> "vence hoy"
        dias == 1L -> "vence mañana"
        else -> "vence en $dias días"
    }
    val fechaTexto = "%02d-%02d-%04d".format(fechaParseada.dayOfMonth, fechaParseada.monthValue, fechaParseada.year)
    return "$cuenta ($fechaTexto)"
}

/// Mismo criterio que `NuevoIngresoModal.tsx` (`confirmarIngreso`): la placa
/// sólo se manda (y sólo existe) cuando el medio es Vehículo -- con
/// Caminando se descarta lo tipeado, aunque el operador haya escrito algo
/// antes de cambiar de radio.
fun placaSiCorresponde(medio: MedioIngreso, placaTexto: String): String? =
    if (medio == MedioIngreso.VEHICULO) placaTexto.trim() else null

/// A diferencia de `PantallaActivos`/`PantallaLogin`, esta pantalla se
/// queda con `remember`/`rememberSaveable` en vez de un `ViewModel` — a
/// propósito, ver mobile/android/arquitectura.md sobre cuándo uno hace falta y
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
    secretoStore: SecretoDispositivoStore,
    preparacion: PreparacionIngreso,
    onRegistrado: () -> Unit,
    onCambiar: () -> Unit,
) {
    var medio by rememberSaveable { mutableStateOf(MedioIngreso.CAMINANDO) }
    var gafeteTexto by rememberSaveable { mutableStateOf("") }
    var placaTexto by rememberSaveable { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var enviando by remember { mutableStateOf(false) }
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

    fun registrarIngreso(gafete: Long?, placa: String?) {
        if (enviando) return
        error = null
        enviando = true
        alcance.launch {
            try {
                withContext(Dispatchers.IO) {
                    // Con gafete, hace falta el secreto para el chequeo
                    // cruzado entre dispositivos del sitio -- sin él no
                    // hay forma de descartar que otro ya lo esté usando,
                    // mismo criterio que antes. Sin gafete, no hace falta
                    // ningún secreto -- `registrarIngresoConSecreto` sólo
                    // toca la red cuando `gafete != null`.
                    val secreto = if (gafete != null) {
                        secretoStore.cargar() ?: throw SecretoDispositivoNoEncontradoException()
                    } else {
                        secretoStore.cargar().orEmpty()
                    }
                    // Chequeo de "gafete ocupado en otro dispositivo del
                    // sitio" y escritura en una sola llamada -- antes eran
                    // dos cruces FFI separados (`gafeteOcupadoEnSitioConSecreto`
                    // + `registrarIngreso`), con una ventana entre medio
                    // donde otro dispositivo podía colarse.
                    nucleo.registrarIngresoConSecreto(preparacion.contratistaId, medio, gafete, placa, secreto)
                }
                onRegistrado()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } catch (excepcion: SecretoDispositivoStoreException) {
                error = excepcion.message
            } catch (excepcion: SecretoDispositivoNoEncontradoException) {
                error = excepcion.message
            } finally {
                enviando = false
            }
        }
    }

    if (escanerGafeteAbierto) {
        PantallaEscanearCedula(
            modo = ModoEscaneoDocumento.GAFETE_CONTRATISTA,
            onDocumentoDetectado = { documento ->
                val limpio = documento.numeroDocumento.filter(Char::isDigit)
                gafeteTexto = limpio
                error = null
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
                    registrarIngreso(gafete, placaSiCorresponde(medio, placaTexto))
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

        if (preparacion.resultadoAcceso == ResultadoAcceso.PermitidoConAdvertencia) {
            val fecha = preparacion.fechaVencimientoPraind
            Text(
                "⚠ PRAIND " + (fecha?.let { mensajeVencimientoPraind(it) } ?: "próximo a vencer"),
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
                // Caminando -- ver el doc-comment de `placaSiCorresponde`.
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
            onClick = {
                error = null
                val gafete: Long? = if (preparacion.requiereGafete) {
                    val numero = gafeteTexto.trim().toLongOrNull()
                    if (numero == null) {
                        error = if (gafeteTexto.isBlank()) "El gafete es requerido" else "Ingrese un número de gafete válido"
                        return@BotonBrisas
                    }
                    numero
                } else {
                    null
                }
                val placa = placaSiCorresponde(medio, placaTexto)
                if (medio == MedioIngreso.VEHICULO && placa.isNullOrBlank()) {
                    error = "La placa es requerida"
                    return@BotonBrisas
                }
                registrarIngreso(gafete, placa)
            },
            enabled = !enviando && gafeteListo && placaLista,
            modifier = Modifier.fillMaxWidth().focusRequester(focoConfirmar),
        ) {
            Text(if (enviando) "Registrando…" else "Registrar ingreso")
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
