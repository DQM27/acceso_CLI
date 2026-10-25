package com.brisas.controlacceso

import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PersonAdd
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
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.Nucleo

private fun FilaCorreoActiva.clave(): String = when (this) {
    is FilaCorreoActiva.Local -> "local-${registro.id}"
    is FilaCorreoActiva.Remota -> "remota-${remoto.uuid}"
}

/// Sin distinguir mayúsculas ni tildes, igual que lo que se resalta
/// ([rangosCoincidencia]).
private fun FilaCorreoActiva.coincideCon(busqueda: String): Boolean {
    if (busqueda.isBlank()) return true
    val (cedula, nombre, motivo, placa) = when (this) {
        is FilaCorreoActiva.Local -> listOf(registro.cedula, registro.nombre, registro.motivo, registro.placa)
        is FilaCorreoActiva.Remota -> listOf(remoto.cedula, remoto.nombre, remoto.motivo, remoto.placa)
    }
    return listOfNotNull(cedula, nombre, motivo, placa).any { rangosCoincidencia(it, busqueda).isNotEmpty() }
}

private const val AYUDA_BUSQUEDA_CORREO = "Cédula, nombre o motivo"

/// La lista de quien entró por correo, la de la pantalla y la del buscador.
@Composable
private fun ListaCorreoActivos(
    filas: List<FilaCorreoActiva>,
    cargando: Boolean,
    busqueda: String,
    sincronizando: Boolean,
    onSincronizar: () -> Unit,
    contentPadding: PaddingValues,
    onElegir: (FilaCorreoActiva) -> Unit,
) {
    ListaConEstados(
        filas = filas,
        clave = { it.clave() },
        cargando = cargando,
        tituloVacio = if (busqueda.isBlank()) "Nadie ingresó por correo" else "Sin resultados",
        detalleVacio = if (busqueda.isBlank()) "Cuando alguien ingrese, aparece aquí." else "Pruebe con otro dato de la persona.",
        sincronizando = sincronizando,
        onSincronizar = onSincronizar,
        contentPadding = contentPadding,
        iconoVacio = IconosLucide.Sobre,
    ) { fila ->
        FilaCorreoActivo(fila = fila, onConfirmarSalida = { onElegir(fila) })
    }
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
    var buscando by remember { mutableStateOf(false) }
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
                // Pantalla completa: sin barra de secciones encima.
                OcultarBarraSecciones()
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
                    // Se busca con la píldora de abajo, como en todas las pestañas.
                    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
                        viewModel.error?.let { mensaje ->
                            FranjaEstado(mensaje, TipoEstado.ERROR, modifier = Modifier.padding(bottom = 4.dp))
                        }

                        ListaCorreoActivos(
                            filas = viewModel.activos,
                            cargando = viewModel.cargando,
                            busqueda = "",
                            sincronizando = sincronizando,
                            onSincronizar = onSincronizar,
                            contentPadding = PaddingValues(top = 4.dp, bottom = EspacioPieLista),
                            onElegir = { filaParaSalida = it },
                        )
                    }

                    PieFlotante(viewModel.confirmacion, modifier = Modifier.align(Alignment.BottomCenter))
                    // "Nuevo ingreso" va en el botón redondo de abajo y la
                    // búsqueda en la píldora (ver `AccionInferior.kt`).
                    AccionPrincipal(Icons.Default.PersonAdd, "Nuevo ingreso por correo", onClick = { mostrandoFormulario = true })
                    BusquedaPrincipal(ayuda = AYUDA_BUSQUEDA_CORREO, onAbrir = { buscando = true })

                    BuscadorInferior(
                        visible = buscando,
                        texto = busqueda,
                        onCambiar = { busqueda = it },
                        ayuda = AYUDA_BUSQUEDA_CORREO,
                        onCerrar = {
                            buscando = false
                            busqueda = ""
                        },
                    ) {
                        Box(modifier = Modifier.padding(horizontal = 16.dp)) {
                            ListaCorreoActivos(
                                filas = viewModel.activos.filter { it.coincideCon(busqueda) },
                                cargando = viewModel.cargando,
                                busqueda = busqueda,
                                sincronizando = sincronizando,
                                onSincronizar = onSincronizar,
                                contentPadding = PADDING_RESULTADOS,
                                onElegir = { fila ->
                                    filaParaSalida = fila
                                    buscando = false
                                    busqueda = ""
                                },
                            )
                        }
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

    // Mientras se llenan los datos, las conexiones con la nube se calientan
    // (ver `CorreoViewModel.precalentarNube`).
    LaunchedEffect(Unit) { viewModel.precalentarNube() }

    // `imePadding()` en un Column SIN `fillMaxSize`, mismo motivo que en
    // [FormularioNuevoIngresoProveedor].
    PantallaConGestoVolver(onVolver = onVolver) {
    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
        val scrollState = rememberScrollState()
        val scope = rememberCoroutineScope()
        Column(modifier = Modifier.verticalScroll(scrollState).imePadding()) {
            EncabezadoPantalla(titulo = "Ingreso por correo", onVolver = onVolver)

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
                FranjaEstado(mensaje, TipoEstado.ERROR, modifier = Modifier.padding(top = 8.dp), vibrar = true)
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
    val datos = fila.datos()
    DialogoRegistrarSalida(
        nombre = datos.nombre,
        datos = datosIngreso(
            cedula = datos.cedula,
            afiliacion = "Motivo" to datos.motivo,
            placa = datos.placa,
            gafete = datos.gafete,
            fechaHora = datos.ingreso,
            dioIngreso = datos.usuario,
            otroEquipo = datos.remota,
        ),
        onDismiss = onDismiss,
        onConfirmar = { onConfirmar(fila) },
    )
}
