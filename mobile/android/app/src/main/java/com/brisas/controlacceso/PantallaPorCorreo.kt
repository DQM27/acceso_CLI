package com.brisas.controlacceso

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PersonAdd
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.Nucleo

private fun FilaCorreoActiva.clave(): String = when (this) {
    is FilaCorreoActiva.Local -> "local-${registro.id}"
    is FilaCorreoActiva.Remota -> "remota-${remoto.uuid}"
}

private fun FilaCorreoActiva.coincideCon(busqueda: String): Boolean {
    val (cedula, nombre, motivo, placa) = when (this) {
        is FilaCorreoActiva.Local -> listOf(registro.cedula, registro.nombre, registro.motivo, registro.placa)
        is FilaCorreoActiva.Remota -> listOf(remoto.cedula, remoto.nombre, remoto.motivo, remoto.placa)
    }
    return listOfNotNull(cedula, nombre, motivo, placa).any { it.contains(busqueda, ignoreCase = true) }
}

/// Ingreso "por correo": visitas autorizadas por correo (generalmente
/// entrevistas de RH), comodín mientras se termina el módulo de Visitas.
/// Mismo esqueleto que [PantallaProveedores] (pedido del usuario
/// 2026-10-03): buscador + lista de activos (locales y del otro dispositivo
/// del sitio) y un formulario aparte con las mismas tarjetas numeradas --
/// datos de la persona (con escáner de cédula), motivo de la visita, y placa
/// y gafete de visita. Vive bajo la pestaña "Externos" ([PantallaExternos]).
@Composable
fun PantallaPorCorreo(
    nucleo: Nucleo,
    refrescarNube: Int = 0,
    // Tirar de la lista hacia abajo sincroniza con la nube, igual que el botón
    // del encabezado de la pantalla principal (que es quien sabe hacerlo).
    sincronizando: Boolean = false,
    onSincronizar: () -> Unit = {},
) {
    RegistrarPantalla("por_correo")
    val viewModel: CorreoViewModel = viewModel(factory = CorreoViewModel.factory(nucleo))
    // Lo que registró o cerró el otro dispositivo llega por la caché: sin
    // esto la lista no se enteraba hasta volver a entrar (mismo motivo que
    // en [PantallaProveedores]).
    LaunchedEffect(refrescarNube) {
        if (refrescarNube > 0) {
            viewModel.refrescarActivos()
        }
    }
    var mostrandoFormulario by remember { mutableStateOf(false) }
    var gafeteTexto by remember { mutableStateOf("") }
    var busqueda by remember { mutableStateOf("") }
    var escaneando by remember { mutableStateOf(false) }
    var escaneandoPlaca by remember { mutableStateOf(false) }
    var filaParaSalida by remember { mutableStateOf<FilaCorreoActiva?>(null) }

    // Qué panel se ve ahora: la lista, el formulario o uno de los escáneres.
    // La lista y el formulario se abren y cierran con una transición corta; los
    // escáneres cambian al instante (ver `TransicionesPantalla.kt`).
    val panel = panelConFormulario(
        mostrandoFormulario = mostrandoFormulario,
        escaneandoCedula = escaneando,
        escaneandoPlaca = escaneandoPlaca,
    )
    AnimatedContent(
        targetState = panel,
        modifier = Modifier.fillMaxSize(),
        transitionSpec = { transicionDePanel(initialState, targetState) },
        label = "panelPorCorreo",
    ) { mostrado ->
        when (mostrado) {
            PanelConFormulario.ESCANER_CEDULA -> {
                PantallaEscanearCedula(
                    modo = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
                    onDocumentoDetectado = { documento ->
                        escaneando = false
                        val numero = documento.numeroDocumento.filter(Char::isDigit)
                            .ifBlank { documento.numeroDocumento }
                        viewModel.rellenarDesdeDocumento(numero, documento.nombre, documento.apellidos)
                    },
                    onCerrar = { escaneando = false },
                )
            }
            PanelConFormulario.ESCANER_PLACA -> {
                PantallaEscanearVehiculoRuta(
                    onVehiculoDetectado = { detectado ->
                        escaneandoPlaca = false
                        viewModel.cambiarPlaca(placaComoSeImprime(detectado.valor))
                    },
                    onCerrar = { escaneandoPlaca = false },
                    mensajeInicial = "Apunte a la placa del vehículo",
                    mensajePermiso = "Se necesita permiso de cámara para escanear la placa.",
                )
            }
            PanelConFormulario.FORMULARIO -> {
                FormularioNuevoIngresoCorreo(
                    viewModel = viewModel,
                    gafeteTexto = gafeteTexto,
                    onCambiarGafeteTexto = { gafeteTexto = it },
                    onEscanear = { escaneando = true },
                    onEscanearPlaca = { escaneandoPlaca = true },
                    onVolver = {
                        viewModel.cancelarFormulario()
                        gafeteTexto = ""
                        mostrandoFormulario = false
                    },
                )
            }
            PanelConFormulario.LISTA -> Box(modifier = Modifier.fillMaxSize()) {
                    val activosFiltrados = if (busqueda.isBlank()) {
                        viewModel.activos
                    } else {
                        viewModel.activos.filter { it.coincideCon(busqueda) }
                    }

                    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
                        // "Nuevo ingreso" es el botón flotante de abajo (ver
                        // [PieFlotante]).
                        CampoBusquedaBrisas(
                            valor = busqueda,
                            onCambiar = { busqueda = it },
                            ayuda = "Cédula, nombre, motivo…",
                            modifier = Modifier.fillMaxWidth(),
                        )

                        viewModel.error?.let { mensaje ->
                            Text(
                                mensaje,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.error,
                                modifier = Modifier.padding(top = 8.dp),
                            )
                        }

                        ListaConEstados(
                            filas = activosFiltrados,
                            clave = { it.clave() },
                            cargando = viewModel.cargando,
                            tituloVacio = if (busqueda.isBlank()) "Nadie ingresó por correo" else "Sin resultados",
                            detalleVacio = if (busqueda.isBlank()) {
                                "Cuando alguien ingrese, aparece acá."
                            } else {
                                "Pruebe con otro dato de la persona."
                            },
                            sincronizando = sincronizando,
                            onSincronizar = onSincronizar,
                            contentPadding = PaddingValues(top = 10.dp, bottom = EspacioPieLista),
                        ) { fila ->
                            FilaCorreoActivo(fila = fila, onConfirmarSalida = { filaParaSalida = fila })
                        }
                    }

                    PieFlotante(viewModel.confirmacion, modifier = Modifier.align(Alignment.BottomCenter)) {
                        BotonFlotanteBrisas(
                            icono = Icons.Default.PersonAdd,
                            descripcion = "Nuevo ingreso",
                            onClick = { mostrandoFormulario = true },
                        )
                    }

                    DialogoConfirmarSalidaCorreo(
                        fila = filaParaSalida,
                        onDismiss = { filaParaSalida = null },
                        onConfirmar = {
                            viewModel.registrarSalida(it)
                            filaParaSalida = null
                        },
                    )
            }
        }
    }
}

/// Formulario en pantalla completa, con las mismas tarjetas numeradas que
/// [FormularioNuevoIngresoProveedor]: 1 datos de la persona, 2 motivo de la
/// visita, 3 placa (opcional) y gafete de visita.
@Composable
private fun FormularioNuevoIngresoCorreo(
    viewModel: CorreoViewModel,
    gafeteTexto: String,
    onCambiarGafeteTexto: (String) -> Unit,
    onEscanear: () -> Unit,
    onEscanearPlaca: () -> Unit,
    onVolver: () -> Unit,
) {
    val paso1Completo = viewModel.cedula.isNotBlank() && viewModel.nombre.isNotBlank()
    val paso2Completo = viewModel.motivo.isNotBlank()
    val paso3Completo = gafeteTexto.isNotBlank()
    val puedeRegistrar = paso1Completo && paso2Completo && paso3Completo &&
        !viewModel.registrando && !viewModel.cedulaConIngresoActivo

    BackHandler(onBack = onVolver)

    // Mientras se llenan los datos, las conexiones con la nube se calientan
    // (ver `CorreoViewModel.precalentarNube`).
    LaunchedEffect(Unit) { viewModel.precalentarNube() }

    // `imePadding()` en un Column SIN `fillMaxSize`, mismo motivo que en
    // [FormularioNuevoIngresoProveedor].
    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
        val scrollState = rememberScrollState()
        val scope = rememberCoroutineScope()
        Column(modifier = Modifier.verticalScroll(scrollState).imePadding()) {
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(
                    "Nuevo ingreso por correo",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                )
                BotonDiscretoBrisas(onClick = onVolver) {
                    Text("← Volver")
                }
            }

            Column(modifier = Modifier.padding(top = 10.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                PasoDatosProveedor(
                    completado = paso1Completo,
                    cedula = viewModel.cedula,
                    nombre = viewModel.nombre,
                    onCambiarCedula = viewModel::cambiarCedula,
                    onCambiarNombre = viewModel::cambiarNombre,
                    onEscanear = onEscanear,
                    titulo = "Datos de la visita",
                )
                TarjetaPasoProveedor(2, "Motivo de la visita", paso2Completo) {
                    OutlinedTextField(
                        value = viewModel.motivo,
                        onValueChange = viewModel::cambiarMotivo,
                        placeholder = { Text("Ej.: Entrevista RH – a quién visita") },
                        singleLine = true,
                        shape = FormaCampoBrisas,
                        colors = ColoresCampoBrisas(),
                        modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas),
                    )
                }
                PasoVehiculoYGafete(
                    completado = paso3Completo,
                    placa = viewModel.placa,
                    onCambiarPlaca = viewModel::cambiarPlaca,
                    onEscanearPlaca = onEscanearPlaca,
                    gafeteTexto = gafeteTexto,
                    onCambiarGafeteTexto = { onCambiarGafeteTexto(it.filter(Char::isDigit)) },
                    // Lleva el scroll al botón de abajo mientras se abre el
                    // teclado (mismo motivo que en proveedores).
                    onGafeteEnfocado = {
                        scope.launch {
                            repeat(15) {
                                scrollState.animateScrollTo(scrollState.maxValue)
                                delay(30)
                            }
                        }
                    },
                    placeholderGafete = "Número de gafete de visita",
                )
            }

            viewModel.error?.let { mensaje ->
                Text(
                    mensaje,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(top = 8.dp),
                )
            }

            BotonBrisas(
                onClick = {
                    val numero = gafeteTexto.toLongOrNull() ?: return@BotonBrisas
                    viewModel.registrarIngreso(numero) {
                        onCambiarGafeteTexto("")
                        onVolver()
                    }
                },
                enabled = puedeRegistrar,
                modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
            ) {
                Text(if (viewModel.registrando) "Registrando…" else "Registrar ingreso")
            }
        }
    }
}

/// Lo que muestra la tarjeta, igual para una fila local o remota.
private data class DatosFilaCorreo(
    val nombre: String,
    val cedula: String,
    val motivo: String,
    val placa: String?,
    val gafete: Long,
    val ingreso: String,
    val usuario: String,
    val remota: Boolean,
)

private fun FilaCorreoActiva.datos(): DatosFilaCorreo = when (this) {
    is FilaCorreoActiva.Local -> DatosFilaCorreo(
        registro.nombre, registro.cedula, registro.motivo, registro.placa,
        registro.gafeteNumero, registro.fechaHoraIngreso, registro.usuarioIngresoNombre, remota = false,
    )
    is FilaCorreoActiva.Remota -> DatosFilaCorreo(
        remoto.nombre, remoto.cedula, remoto.motivo, remoto.placa,
        remoto.gafeteNumero, remoto.horaEntrada, remoto.usuarioEntradaNombre, remota = true,
    )
}

/// La misma tarjeta que contratistas ([TarjetaActivo]), con el motivo en
/// vez de la empresa y el ícono de la PC en las filas remotas.
@Composable
private fun FilaCorreoActivo(fila: FilaCorreoActiva, onConfirmarSalida: () -> Unit) {
    val (nombre, cedula, motivo, placa, gafete, ingreso, usuario, remota) = fila.datos()
    TarjetaActivo(
        nombre = nombre,
        detalle = "$cedula · $motivo" + (placa?.let { " · $it" } ?: ""),
        gafeteNumero = gafete,
        fechaHoraIngreso = ingreso,
        dioIngreso = usuario,
        otroEquipo = remota,
        onClick = onConfirmarSalida,
    )
}

/// El mismo diálogo que contratistas ([DialogoRegistrarSalida]).
@Composable
private fun DialogoConfirmarSalidaCorreo(
    fila: FilaCorreoActiva?,
    onDismiss: () -> Unit,
    onConfirmar: (FilaCorreoActiva) -> Unit,
) {
    if (fila == null) return
    DialogoRegistrarSalida(
        detalle = when (fila) {
            is FilaCorreoActiva.Local ->
                "${fila.registro.nombre} · ${fila.registro.cedula} · ${fila.registro.motivo} · Gafete ${fila.registro.gafeteNumero}"
            is FilaCorreoActiva.Remota -> "${fila.remoto.nombre} · $TEXTO_OTRO_DISPOSITIVO"
        },
        onDismiss = onDismiss,
        onConfirmar = { onConfirmar(fila) },
    )
}
