package com.brisas.controlacceso

import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.BusquedaActivos
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.PreparacionIngreso
/// Una sola vista para el ciclo completo — entrada, permanencia y salida —,
/// igual que `Activos.tsx` en desktop (ahí "+Nuevo"/"Salida" abren modales
/// sobre la misma grilla de activos; ver docs/plan-app-movil.md, addendum
/// 2026-09-01). La lista es quién está adentro (tocar un nombre confirma su
/// salida). Un solo buscador para personas y gafetes (2026-10-28: antes eran
/// dos pestañas, "Persona" y "Gafete"); qué se busca lo decide el núcleo
/// según lo escrito (`interpretarBusquedaActivos`):
///
/// - **Nombre o cédula**: el catálogo completo de contratistas. Elegir a
///   alguien abre su ingreso o, si ya está adentro, su salida. Si no
///   aparece, se lo registra ahí mismo. La cámara también busca personas.
/// - **Gafetes** (de 1 a 3 dígitos, varios separados por coma: "2, 25,
///   85"): a quién le corresponde cada uno, con la tarjeta de la lista en
///   azul de salida. Tocar una abre su salida; con dos o más, un botón los
///   saca a todos de una vez (la vista previa con nombres hace de
///   confirmación, como en escritorio).
///
/// Se busca siempre con la píldora de abajo ([BuscadorInferior], rediseño
/// 2026-10-25).
///
/// Todo el estado y las llamadas a [Nucleo] viven en [ActivosViewModel]
/// (ver mobile/android/arquitectura.md) — este archivo sólo dibuja lo que el
/// ViewModel expone y le reporta eventos. Esta función orquesta: delega los
/// mensajes, la lista y los resultados del buscador (ver
/// [ContenidoListaActivos]/[ResultadosIngreso]/[ResultadosGafete] en
/// `ContenidoActivos.kt`) a funciones chicas de una sola responsabilidad.
@Composable
fun PantallaActivos(
    nucleo: Nucleo,
    refrescarNube: Int = 0,
    // Tirar de la lista hacia abajo sincroniza con la nube, igual que el botón
    // del encabezado de la pantalla principal (que es quien sabe hacerlo).
    sincronizando: Boolean = false,
    onSincronizar: () -> Unit = {},
    // Registrar a quien no aparece en la búsqueda (con lo escrito o lo
    // leído): lo abre la pantalla principal, que es la dueña de "Nuevo
    // contratista".
    onNuevoContratista: (String) -> Unit = {},
) {
    RegistrarPantalla("activos")
    val viewModel: ActivosViewModel =
        viewModel(factory = ActivosViewModel.factory(nucleo))
    var escanerAbierto by remember { mutableStateOf(false) }
    // El buscador de abajo (la píldora de búsqueda).
    var buscando by remember { mutableStateOf(false) }
    LaunchedEffect(refrescarNube) {
        if (refrescarNube > 0) {
            viewModel.refrescar()
        }
    }
    // Tras escanear un documento: denegado, advertencia o no encontrado
    // suenan y vibran (ver `AlertaEscaneo.kt`).
    val contexto = LocalContext.current
    LaunchedEffect(viewModel.alertaEscaneo?.id) {
        val aviso = viewModel.alertaEscaneo ?: return@LaunchedEffect
        sonarAlertaEscaneo(contexto, aviso.alerta)
        // Sonada una vez: volver a la pestaña no la repite.
        viewModel.alertaSonada(aviso.id)
    }

    // El ingreso es una hoja sobre la lista ([HojaIngreso]), no un panel: se
    // sigue dibujando mientras baja, aunque el ViewModel ya haya soltado la
    // selección. Lo escrito vive acá, no en la hoja: la cámara de placa la
    // tapa (la saca de la composición) y al volver tiene que estar igual.
    val formulario = viewModel.seleccionIngreso as? SeleccionIngreso.Formulario
    var hojaIngreso by remember { mutableStateOf<PreparacionIngreso?>(null) }
    LaunchedEffect(formulario) {
        if (formulario != null) hojaIngreso = formulario.preparacion
    }
    val camposIngreso = rememberCamposIngreso(hojaIngreso?.contratistaId)
    var escanerPlacaAbierto by remember { mutableStateOf(false) }

    // Qué panel se ve ahora (mismo orden de prioridad de siempre: primero el
    // ingreso bloqueado, luego los escáneres, y si no la lista).
    val panel: PanelActivos = when (val actual = viewModel.seleccionIngreso) {
        is SeleccionIngreso.Bloqueada -> PanelActivos.Bloqueado(actual)
        else -> when {
            escanerPlacaAbierto && formulario != null -> PanelActivos.EscanerPlaca
            escanerAbierto -> PanelActivos.EscanerCedula
            else -> PanelActivos.Lista
        }
    }

    // Los paneles sin cámara se abren y cierran con una transición corta; los
    // de cámara cambian al instante (ver `TransicionesPantalla.kt`). La clave
    // es la clase del panel: un cambio dentro del mismo panel (por ejemplo el
    // error del formulario) se actualiza en el lugar, sin animar.
    AnimatedContent(
        targetState = panel,
        modifier = Modifier.fillMaxSize(),
        contentKey = { it::class },
        transitionSpec = {
            if (initialState.conCamara || targetState.conCamara) {
                SinTransicion
            } else {
                transicionDePantalla(avanza = targetState !is PanelActivos.Lista)
            }
        },
        label = "panelActivos",
    ) { mostrado ->
        when (mostrado) {
            is PanelActivos.Bloqueado -> {
                // Ocupa toda la pantalla: sin barra de secciones encima (las
                // cámaras la esconden solas).
                OcultarBarraSecciones()
                PantallaIngresoBloqueado(
                    preparacion = mostrado.seleccion.preparacion,
                    mensaje = mostrado.seleccion.mensaje,
                    onCambiar = { viewModel.cancelarSeleccionIngreso() },
                    salida = mostrado.seleccion.salidaOtraVia,
                    salidaOtraVia = viewModel.salidaOtraVia,
                    onRegistrarSalidaOtraVia = viewModel::registrarSalidaPorOtraVia,
                )
            }
            PanelActivos.EscanerCedula -> {
                PantallaEscanearCedula(
                    modo = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
                    onDocumentoDetectado = { documento ->
                        // La cámara se queda hasta que la confirmación está lista
                        // y entonces pasa directo a ella: cerrarla antes dejaba
                        // ver la lista un instante en medio. `finally`: si algo
                        // cancela la espera, el escáner igual se cierra.
                        try {
                            viewModel.usarDocumentoEscaneadoIngreso(documento).join()
                        } finally {
                            escanerAbierto = false
                        }
                        // Si el documento no llevó a nadie con claridad, lo leído
                        // queda en el buscador de abajo, con sus resultados, para
                        // corregirlo, elegir a mano o registrarlo. Si ya estaba
                        // adentro, su salida ya está abierta.
                        if (viewModel.seleccionIngreso == SeleccionIngreso.Ninguna &&
                            viewModel.seleccionSalida == null &&
                            viewModel.texto.isNotBlank()
                        ) {
                            buscando = true
                        }
                    },
                    onCerrar = { escanerAbierto = false },
                )
            }
            PanelActivos.EscanerPlaca -> {
                // La misma cámara que lee placas en Proveedores y Rutas. Si
                // lee un número de unidad (calcomanía de flota) en vez de una
                // placa, igual se usa el texto leído, mismo criterio que
                // Proveedores. Al cerrarla vuelve la hoja de ingreso.
                PantallaEscanearVehiculoRuta(
                    onVehiculoDetectado = { detectado ->
                        // Como va impresa en la placa (`CL 371931`, `BPH-485`).
                        camposIngreso.placaTexto = placaComoSeImprime(detectado.valor)
                        viewModel.limpiarErrorIngreso()
                        escanerPlacaAbierto = false
                    },
                    onCerrar = { escanerPlacaAbierto = false },
                    mensajeInicial = "Apunte a la placa del vehículo",
                    mensajePermiso = "Se necesita permiso de cámara para escanear la placa.",
                )
            }
            PanelActivos.Lista -> Box(modifier = Modifier.fillMaxSize()) {
            val verificando = viewModel.seleccionIngreso is SeleccionIngreso.Cargando

            // Arriba sólo los mensajes y la lista: se busca con la píldora de
            // abajo, igual en todas las pestañas, al alcance del pulgar.
            // Sin relleno arriba: las tarjetas llegan hasta la línea de la
            // barra de arriba al desplazar (pedido del usuario 2026-10-29); el
            // aire de arriba lo da el `contentPadding` de la lista.
            Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp)) {
                MensajesEstado(
                    error = viewModel.error,
                    mensaje = viewModel.mensaje,
                    mensajeEsError = viewModel.mensajeEsError,
                    verificando = verificando,
                )

                ContenidoListaActivos(
                    activos = viewModel.activos,
                    cargando = viewModel.cargando,
                    onElegirActivo = { viewModel.elegirSeleccionSalida(it) },
                    sincronizando = sincronizando,
                    onSincronizar = onSincronizar,
                )
            }

            AccionActivos(onEscanearCedula = { escanerAbierto = true })
            BusquedaPrincipal(ayuda = AYUDA_BUSQUEDA_ACTIVOS, onAbrir = { buscando = true })

            // El buscador desde abajo, como en iOS. Elegir a alguien abre su
            // panel y cierra la búsqueda.
            val cerrarBusqueda = {
                buscando = false
                viewModel.cambiarTexto("")
            }
            BuscadorInferior(
                visible = buscando,
                texto = viewModel.texto,
                onCambiar = { viewModel.cambiarTexto(it) },
                ayuda = AYUDA_BUSQUEDA_ACTIVOS,
                onCerrar = cerrarBusqueda,
            ) {
                when (viewModel.busqueda) {
                    BusquedaActivos.Vacia, BusquedaActivos.Personas -> Box(modifier = Modifier.padding(horizontal = 16.dp)) {
                        ResultadosIngreso(
                            texto = viewModel.texto,
                            resultadosBusqueda = viewModel.resultadosBusqueda,
                            cargando = viewModel.cargando,
                            onElegirContratista = { contratista ->
                                // `elegir` no depende del texto: limpiarlo no lo corta.
                                viewModel.elegir(contratista.id)
                                cerrarBusqueda()
                            },
                            onNuevoContratista = { escrito ->
                                cerrarBusqueda()
                                onNuevoContratista(escrito)
                            },
                        )
                    }
                    is BusquedaActivos.Gafetes -> ResultadosGafete(
                        coincidencias = viewModel.coincidenciasGafete,
                        enviando = viewModel.enviandoGafetes,
                        onElegir = { fila ->
                            viewModel.elegirSeleccionSalida(fila)
                            cerrarBusqueda()
                        },
                        onRegistrarSalidaGafetes = {
                            // Toma las coincidencias en el acto; el resultado
                            // queda en la franja de arriba.
                            viewModel.registrarSalidaPorGafetes()
                            buscando = false
                        },
                    )
                }
            }

            DialogoConfirmarSalida(
                fila = viewModel.seleccionSalida,
                onDismiss = { viewModel.elegirSeleccionSalida(null) },
                onConfirmar = { viewModel.confirmarSalida(it) },
                yaAdentro = viewModel.salidaYaAdentro,
            )

            hojaIngreso?.let { preparacion ->
                HojaIngreso(
                    preparacion = preparacion,
                    campos = camposIngreso,
                    abierta = formulario?.preparacion == preparacion,
                    error = viewModel.errorIngreso,
                    registrando = viewModel.registrandoIngreso,
                    onRegistrar = viewModel::registrarIngreso,
                    onLimpiarError = viewModel::limpiarErrorIngreso,
                    onCancelar = { viewModel.cancelarSeleccionIngreso() },
                    onEscanearPlaca = { escanerPlacaAbierto = true },
                    onCerrada = { hojaIngreso = null },
                )
            }

            }
        }
    }
}

/// Qué ocupa la pantalla de Activos (ver [PantallaActivos]). Los paneles de
/// cámara no se animan al abrir ni al cerrar.
private sealed interface PanelActivos {
    val conCamara: Boolean get() = false

    data object Lista : PanelActivos
    data class Bloqueado(val seleccion: SeleccionIngreso.Bloqueada) : PanelActivos
    data object EscanerPlaca : PanelActivos {
        override val conCamara get() = true
    }
    data object EscanerCedula : PanelActivos {
        override val conCamara get() = true
    }
}
