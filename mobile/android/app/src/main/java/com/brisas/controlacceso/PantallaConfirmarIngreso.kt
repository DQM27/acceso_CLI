package com.brisas.controlacceso

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.listSaver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.selection.toggleable
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import uniffi.control_acceso_mobile.MedioIngreso
import uniffi.control_acceso_mobile.PreparacionIngreso

/// Lo que lleva escrito el formulario de ingreso. Vive afuera de
/// [HojaIngreso] (lo guarda [PantallaActivos]): la cámara de placa ocupa
/// toda la pantalla y la hoja se va mientras tanto, y al volver tiene que
/// estar todo como estaba. Sobrevive también a una rotación.
internal class CamposIngreso {
    var medio by mutableStateOf(MedioIngreso.CAMINANDO)
    var gafeteTexto by mutableStateOf("")

    // "Sin gafete" (S/G), pedido del usuario 2026-10-03: cualquier operador
    // puede registrar sin gafete, sin motivo, igual que el personal de ruta.
    var sinGafete by mutableStateOf(false)
    var placaTexto by mutableStateOf("")

    companion object {
        val Saver = listSaver<CamposIngreso, Any>(
            save = { listOf(it.medio.name, it.gafeteTexto, it.sinGafete, it.placaTexto) },
            restore = { guardado ->
                CamposIngreso().apply {
                    medio = MedioIngreso.valueOf(guardado[0] as String)
                    gafeteTexto = guardado[1] as String
                    sinGafete = guardado[2] as Boolean
                    placaTexto = guardado[3] as String
                }
            },
        )
    }
}

/// Un formulario fresco por contratista ([clave]).
@Composable
internal fun rememberCamposIngreso(clave: Any?): CamposIngreso =
    rememberSaveable(clave, saver = CamposIngreso.Saver) { CamposIngreso() }

/// Formulario de ingreso de un contratista ya preparado, como hoja que sube
/// desde abajo sobre la lista, igual que la de salida (pedido del usuario
/// 2026-10-27: en pantalla completa, sin gafete, el botón quedaba arriba,
/// lejos del pulgar). Con el teclado abierto la hoja sube con él y el botón
/// queda justo encima. La distingue de la salida la flecha que avanza hacia
/// la derecha ([IconoPaso]).
///
/// El registro (la validación de gafete/placa y la llamada al núcleo) vive en
/// [ActivosViewModel.registrarIngreso] -- punto M2 de la auditoría móvil.
/// Acá sólo se dibujan los [campos].
///
/// [abierta] la decide el ViewModel: cuando suelta la selección (se registró
/// o se canceló) la hoja baja y recién entonces avisa con [onCerrada], así no
/// desaparece de golpe. Si se registró, la flecha sigue de largo mientras baja.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun HojaIngreso(
    preparacion: PreparacionIngreso,
    campos: CamposIngreso,
    abierta: Boolean,
    error: String?,
    registrando: Boolean,
    onRegistrar: (medio: MedioIngreso, gafeteTexto: String, placaTexto: String) -> Unit,
    onLimpiarError: () -> Unit,
    onCancelar: () -> Unit,
    onEscanearPlaca: () -> Unit,
    onCerrada: () -> Unit,
) {
    RegistrarPantalla("confirmar_ingreso")
    val estado = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val teclado = LocalSoftwareKeyboardController.current
    val hapticos = rememberHapticos()
    // Se tocó "Registrar ingreso" y todavía no volvió un error: si la hoja se
    // cierra así, fue porque entró.
    var registrado by remember { mutableStateOf(false) }
    LaunchedEffect(error) {
        if (error != null) registrado = false
    }
    LaunchedEffect(abierta) {
        if (abierta) return@LaunchedEffect
        teclado?.hide()
        if (registrado) delay(MS_ANTES_DE_BAJAR)
        estado.hide()
        onCerrada()
    }

    // Mismo criterio que [PantallaProveedores]/[PantallaRutas]: el botón
    // se ve gris (deshabilitado) mientras falten datos, en vez de dejar
    // que el error salga recién al tocarlo -- pedido explícito del
    // usuario 2026-09-20, para que las dos pantallas se sientan igual.
    // Cuando el contratista no requiere gafete no hay nada más que
    // completar (el medio de ingreso ya arranca con un valor elegido),
    // así que el botón queda habilitado de entrada. Si lo requiere, hace
    // falta el número o encender "S/G": nunca queda S/G por olvido.
    val gafeteListo = !preparacion.requiereGafete || campos.sinGafete || campos.gafeteTexto.trim().toLongOrNull() != null
    val placaLista = campos.medio != MedioIngreso.VEHICULO || campos.placaTexto.isNotBlank()
    val puedeRegistrar = abierta && !registrando && gafeteListo && placaLista
    // La tecla ✓ del teclado registra (el gafete suele ser lo último que se
    // escribe): sin bajar la vista a buscar el botón.
    val registrarSiListo = {
        if (puedeRegistrar) {
            registrado = true
            onRegistrar(campos.medio, campos.gafeteTexto, campos.placaTexto)
        }
    }

    val arriba = rememberHojaArriba(estado)
    ModalBottomSheet(
        onDismissRequest = onCancelar,
        sheetState = estado,
        containerColor = MaterialTheme.colorScheme.background,
    ) {
        val focoGafete = remember { FocusRequester() }
        // El teclado del gafete se abre solo, pero recién cuando la hoja
        // terminó de subir y la flecha de entrar: las tres cosas a la vez
        // (hoja, flecha y teclado acomodando la hoja cuadro a cuadro) se
        // trababan y la flecha no se llegaba a ver.
        LaunchedEffect(arriba) {
            if (arriba && preparacion.requiereGafete && !campos.sinGafete && campos.gafeteTexto.isEmpty()) {
                delay(MS_FLECHA.toLong())
                runCatching { focoGafete.requestFocus() }
            }
        }
        // Si lo de abajo no entra (nombre largo, vehículo con placa y el
        // teclado abierto), la hoja se corre sola hasta el fondo: los botones
        // nunca quedan tapados. Sólo se mueve cuando cambia lo que sobra.
        val desplazamiento = rememberScrollState()
        LaunchedEffect(desplazamiento) {
            snapshotFlow { desplazamiento.maxValue }.collect { sobra ->
                if (sobra > 0) desplazamiento.scrollTo(sobra)
            }
        }
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(desplazamiento)
                .padding(start = 20.dp, end = 20.dp, bottom = 24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            // En fila y no centrado uno debajo del otro como en la salida:
            // con el teclado del gafete abierto no sobra alto, y un nombre de
            // dos líneas dejaba "Cancelar" debajo del teclado (2026-10-27).
            Row(
                modifier = Modifier.fillMaxWidth().padding(bottom = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                IconoPaso(Paso.INGRESO, arriba = arriba, siguiendo = registrado && !abierta, tamano = 48.dp)
                Column(modifier = Modifier.weight(1f).padding(start = 14.dp)) {
                    Text(preparacion.nombre, style = MaterialTheme.typography.titleLarge)
                    Text(
                        "${preparacion.cedula} · ${preparacion.empresaNombre}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(top = 2.dp),
                    )
                }
            }

            // Texto y cuenta de días del núcleo (`aviso_praind`, con su reloj).
            preparacion.avisoPraind?.let { aviso ->
                FranjaEstado(aviso, TipoEstado.AVISO, modifier = Modifier.fillMaxWidth().padding(bottom = 12.dp))
            }
            if (preparacion.gafetesDeuda.isNotEmpty()) {
                FranjaEstado(
                    "Este contratista debe el gafete " +
                        preparacion.gafetesDeuda.joinToString(", ") { "#${it.toString().padStart(2, '0')}" },
                    TipoEstado.AVISO,
                    modifier = Modifier.fillMaxWidth().padding(bottom = 12.dp),
                )
            }

            // El mismo control segmentado que el resto de la app, en vez de dos
            // botones de radio de Material. Sin la etiqueta "Medio de
            // ingreso" que llevaba encima: "Caminando | Vehículo" se explica
            // solo y ese alto hacía falta con el teclado abierto.
            val medios = listOf(MedioIngreso.CAMINANDO, MedioIngreso.VEHICULO)
            FilaPildoras(
                opciones = listOf("Caminando", "Vehículo"),
                seleccionado = medios.indexOf(campos.medio),
                onSeleccionar = { indice ->
                    campos.medio = medios[indice]
                    // Descarta la placa tipeada antes si el operador vuelve a
                    // Caminando: el núcleo descarta la placa (`placa_segun_medio`).
                    if (campos.medio == MedioIngreso.CAMINANDO) campos.placaTexto = ""
                },
                modifier = Modifier.fillMaxWidth().padding(bottom = 16.dp),
            )

            if (campos.medio == MedioIngreso.VEHICULO) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(bottom = 16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    OutlinedTextField(
                        value = campos.placaTexto,
                        onValueChange = { campos.placaTexto = it },
                        label = { Text("Placa del vehículo") },
                        singleLine = true,
                        // Con gafete por escribir, ✓ pasa a él; si no, registra.
                        keyboardOptions = KeyboardOptions(
                            imeAction = if (preparacion.requiereGafete && !campos.sinGafete) ImeAction.Next else ImeAction.Done,
                        ),
                        keyboardActions = KeyboardActions(
                            onNext = { focoGafete.requestFocus() },
                            onDone = { registrarSiListo() },
                        ),
                        shape = FormaCampoBrisas,
                        colors = ColoresCampoBrisas(),
                        modifier = Modifier.weight(1f),
                    )
                    // La misma cámara que lee placas en Proveedores y Rutas
                    // (`LectorVehiculoRuta.kt`), pedido del usuario 2026-10-21.
                    // La abre [PantallaActivos], que tapa la hoja mientras tanto.
                    // `top` por la etiqueta flotante del campo, que corre su
                    // marco hacia abajo: así el botón queda a la par del marco.
                    BotonCuadradoBordeBrisas(
                        icono = Icons.Default.PhotoCamera,
                        descripcion = "Escanear placa",
                        onClick = onEscanearPlaca,
                        tamano = 56.dp,
                        modifier = Modifier.padding(start = 8.dp, top = 8.dp),
                    )
                }
            }

            if (preparacion.requiereGafete) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(bottom = 16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    OutlinedTextField(
                        value = campos.gafeteTexto,
                        onValueChange = { campos.gafeteTexto = it.filter(Char::isDigit) },
                        label = { Text("Número de gafete") },
                        singleLine = true,
                        enabled = !campos.sinGafete,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Done),
                        keyboardActions = KeyboardActions(onDone = { registrarSiListo() }),
                        shape = FormaCampoBrisas,
                        colors = ColoresCampoBrisas(),
                        modifier = Modifier.weight(1f).focusRequester(focoGafete),
                    )
                    // "S/G" de dos estados en lugar de la cámara (escanear el
                    // número del gafete casi no se usaba y estorbaba) y de la
                    // casilla "Sin gafete" que iba debajo (pedido del usuario
                    // 2026-10-21). Encenderlo borra el número tipeado: sin número
                    // el núcleo registra el ingreso S/G.
                    BotonSinGafete(
                        activo = campos.sinGafete,
                        onAlternar = {
                            campos.sinGafete = !campos.sinGafete
                            hapticos(if (campos.sinGafete) Haptico.ACTIVAR else Haptico.DESACTIVAR)
                            if (campos.sinGafete) campos.gafeteTexto = ""
                            onLimpiarError()
                        },
                        modifier = Modifier.padding(start = 8.dp, top = 8.dp),
                    )
                }
            }

            val mensajeError = error
            if (mensajeError != null) {
                FranjaEstado(
                    mensajeError,
                    TipoEstado.ERROR,
                    modifier = Modifier.fillMaxWidth().padding(bottom = 12.dp),
                    vibrar = true,
                )
            }

            BotonBrisas(
                onClick = registrarSiListo,
                enabled = puedeRegistrar,
                modifier = Modifier.fillMaxWidth().padding(top = 4.dp),
            ) {
                Text(if (registrando) "Registrando…" else "Registrar ingreso")
            }
            BotonDiscretoBrisas(onClick = onCancelar, modifier = Modifier.padding(top = 4.dp)) {
                Text("Cancelar")
            }
        }
    }
}

@Composable
fun PantallaIngresoBloqueado(preparacion: PreparacionIngreso, mensaje: String, onCambiar: () -> Unit) {
    PantallaBloqueoIngreso(
        titulo = preparacion.nombre,
        subtitulo = "${preparacion.cedula} · ${preparacion.empresaNombre}",
        mensaje = mensaje,
        onVolver = onCambiar,
    )
}

/// La persona no puede entrar: su nombre y, en una franja de error, el
/// motivo. La misma para contratistas, proveedores y por correo (pedido del
/// usuario 2026-10-26: que los tres frenen igual apenas se escanea).
@Composable
internal fun PantallaBloqueoIngreso(titulo: String, subtitulo: String, mensaje: String, onVolver: () -> Unit) {
    PantallaConGestoVolver(onVolver = onVolver) {
        Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
            EncabezadoPantalla(
                titulo = titulo,
                subtitulo = subtitulo,
                onVolver = onVolver,
                modifier = Modifier.padding(bottom = 16.dp),
            )
            // En una franja de error y en negrita -- pedido explícito del
            // usuario 2026-09-20: que el motivo de la denegación se lea con
            // más fuerza que el resto del texto. Sin toque de rechazo: tras un
            // escaneo ya suena y vibra la alerta (`AlertaEscaneo.kt`).
            FranjaEstado(mensaje, TipoEstado.ERROR)
        }
    }
}

/// Botón de dos estados "S/G" (sin gafete), del mismo alto que el campo del
/// número: apagado, con borde y letras en el color de la sección; encendido,
/// relleno. El cambio de un estado al otro se funde.
@Composable
private fun BotonSinGafete(activo: Boolean, onAlternar: () -> Unit, modifier: Modifier = Modifier) {
    val acento = MaterialTheme.colorScheme.primary
    val fondo by animateColorAsState(if (activo) acento else Color.Transparent, tween(MS_FUNDIDO_ENTRADA), label = "fondoSinGafete")
    val texto by animateColorAsState(
        if (activo) MaterialTheme.colorScheme.onPrimary else acento,
        tween(MS_FUNDIDO_ENTRADA),
        label = "textoSinGafete",
    )
    Box(
        modifier = modifier
            .size(width = 64.dp, height = 56.dp)
            .clip(FormaCampoBrisas)
            .background(fondo)
            .border(1.dp, acento, FormaCampoBrisas)
            .toggleable(value = activo, role = Role.Switch, onValueChange = { onAlternar() })
            .semantics { contentDescription = "Sin gafete" },
        contentAlignment = Alignment.Center,
    ) {
        Text("S/G", color = texto, fontWeight = FontWeight.Bold)
    }
}
