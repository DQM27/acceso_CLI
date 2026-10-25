package com.brisas.controlacceso

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.selection.toggleable
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
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
    // "Sin gafete" (S/G), pedido del usuario 2026-10-03: cualquier operador
    // puede registrar sin gafete, sin motivo, igual que el personal de ruta.
    var sinGafete by rememberSaveable { mutableStateOf(false) }
    var placaTexto by rememberSaveable { mutableStateOf("") }
    var escanerPlacaAbierto by remember { mutableStateOf(false) }
    val alcance = rememberCoroutineScope()
    val hapticos = rememberHapticos()

    // La misma cámara que lee placas en Proveedores y Rutas
    // (`LectorVehiculoRuta.kt`), pedido del usuario 2026-10-21. Si lee un
    // número de unidad (calcomanía de flota) en vez de una placa, igual se usa
    // el texto leído, mismo criterio que Proveedores.
    if (escanerPlacaAbierto) {
        PantallaEscanearVehiculoRuta(
            onVehiculoDetectado = { detectado ->
                // Como va impresa en la placa (`CL 371931`, `BPH-485`).
                placaTexto = placaComoSeImprime(detectado.valor)
                onLimpiarError()
                escanerPlacaAbierto = false
            },
            onCerrar = { escanerPlacaAbierto = false },
            mensajeInicial = "Apunte a la placa del vehículo",
            mensajePermiso = "Se necesita permiso de cámara para escanear la placa.",
        )
        return
    }

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

    // Mismo criterio que [PantallaRutas]/[PantallaProveedores]/
    // [PantallaNuevoContratista]: `imePadding()` va en un Column SIN
    // `fillMaxSize` -- combinarlo con `fillMaxSize` deja un hueco enorme
    // entre el teclado y el contenido (bug reportado 2026-09-20).
    val scrollStateFormulario = rememberScrollState()
    // Atrás del sistema (con su gesto) vuelve a la lista, igual que
    // "‹ Volver"; con el escáner de placa abierto manda el de la cámara.
    PantallaConGestoVolver(onVolver = onCambiar, habilitado = !escanerPlacaAbierto) {
    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
    Column(
        modifier = Modifier.verticalScroll(scrollStateFormulario).imePadding(),
    ) {
        // El mismo encabezado que el resto de las pantallas que se abren
        // encima de otra: "‹ Volver" arriba y el nombre en grande.
        EncabezadoPantalla(
            titulo = preparacion.nombre,
            subtitulo = "${preparacion.cedula} · ${preparacion.empresaNombre}",
            onVolver = onCambiar,
            modifier = Modifier.padding(bottom = 16.dp),
        )

        // Texto y cuenta de días del núcleo (`aviso_praind`, con su reloj).
        preparacion.avisoPraind?.let { aviso ->
            FranjaEstado(aviso, TipoEstado.AVISO, modifier = Modifier.padding(bottom = 12.dp))
        }
        if (preparacion.gafetesDeuda.isNotEmpty()) {
            FranjaEstado(
                "Este contratista debe el gafete " +
                    preparacion.gafetesDeuda.joinToString(", ") { "#${it.toString().padStart(2, '0')}" },
                TipoEstado.AVISO,
                modifier = Modifier.padding(bottom = 12.dp),
            )
        }

        Text(
            "Medio de ingreso",
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(start = 4.dp, bottom = 6.dp),
        )
        // El mismo control segmentado que el resto de la app, en vez de dos
        // botones de radio de Material.
        val medios = listOf(MedioIngreso.CAMINANDO, MedioIngreso.VEHICULO)
        FilaPildoras(
            opciones = listOf("Caminando", "Vehículo"),
            seleccionado = medios.indexOf(medio),
            onSeleccionar = { indice ->
                medio = medios[indice]
                // Descarta la placa tipeada antes si el operador vuelve a
                // Caminando: el núcleo descarta la placa (`placa_segun_medio`).
                if (medio == MedioIngreso.CAMINANDO) placaTexto = ""
            },
            modifier = Modifier.padding(bottom = 16.dp),
        )

        if (medio == MedioIngreso.VEHICULO) {
            Row(
                modifier = Modifier.fillMaxWidth().padding(bottom = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedTextField(
                    value = placaTexto,
                    onValueChange = { placaTexto = it },
                    label = { Text("Placa del vehículo") },
                    singleLine = true,
                    shape = FormaCampoBrisas,
                    colors = ColoresCampoBrisas(),
                    modifier = Modifier.weight(1f),
                )
                // `top` por la etiqueta flotante del campo, que corre su
                // marco hacia abajo: así el botón queda a la par del marco.
                BotonCuadradoBordeBrisas(
                    icono = Icons.Default.PhotoCamera,
                    descripcion = "Escanear placa",
                    onClick = { escanerPlacaAbierto = true },
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
                    value = gafeteTexto,
                    onValueChange = { gafeteTexto = it.filter(Char::isDigit) },
                    label = { Text("Número de gafete") },
                    singleLine = true,
                    enabled = !sinGafete,
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
                // "S/G" de dos estados en lugar de la cámara (escanear el
                // número del gafete casi no se usaba y estorbaba) y de la
                // casilla "Sin gafete" que iba debajo (pedido del usuario
                // 2026-10-21). Encenderlo borra el número tipeado: sin número
                // el núcleo registra el ingreso S/G.
                BotonSinGafete(
                    activo = sinGafete,
                    onAlternar = {
                        sinGafete = !sinGafete
                        hapticos(if (sinGafete) Haptico.ACTIVAR else Haptico.DESACTIVAR)
                        if (sinGafete) gafeteTexto = ""
                        onLimpiarError()
                    },
                    modifier = Modifier.padding(start = 8.dp, top = 8.dp),
                )
            }
        }

        val mensajeError = error
        if (mensajeError != null) {
            FranjaEstado(mensajeError, TipoEstado.ERROR, modifier = Modifier.padding(bottom = 12.dp), vibrar = true)
        }

        // Mismo criterio que [PantallaProveedores]/[PantallaRutas]: el botón
        // se ve gris (deshabilitado) mientras falten datos, en vez de dejar
        // que el error salga recién al tocarlo -- pedido explícito del
        // usuario 2026-09-20, para que las dos pantallas se sientan igual.
        // Cuando el contratista no requiere gafete no hay nada más que
        // completar (el medio de ingreso ya arranca con un valor elegido),
        // así que el botón queda habilitado de entrada. Si lo requiere, hace
        // falta el número o encender "S/G": nunca queda S/G por olvido.
        val gafeteListo = !preparacion.requiereGafete || sinGafete || gafeteTexto.trim().toLongOrNull() != null
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
}

@Composable
fun PantallaIngresoBloqueado(preparacion: PreparacionIngreso, mensaje: String, onCambiar: () -> Unit) {
    PantallaConGestoVolver(onVolver = onCambiar) {
        Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
            // El mismo encabezado que [PantallaConfirmarIngreso].
            EncabezadoPantalla(
                titulo = preparacion.nombre,
                subtitulo = "${preparacion.cedula} · ${preparacion.empresaNombre}",
                onVolver = onCambiar,
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
