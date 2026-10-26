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
import uniffi.control_acceso_mobile.Nucleo
/// Una sola vista para el ciclo completo — entrada, permanencia y salida —,
/// igual que `Activos.tsx` en desktop (ahí "+Nuevo"/"Salida" abren modales
/// sobre la misma grilla de activos; ver docs/plan-app-movil.md, addendum
/// 2026-09-01). Acá no hay una pestaña "Buscar" aparte: el mismo campo de
/// texto cambia de sentido según el modo elegido en el selector de arriba
/// — mismo espíritu que el checkbox "Por gafete" de `SalidaModal.tsx` (un
/// solo campo, la interpretación cambia):
///
/// - **Persona** (por defecto, `ModoBusqueda.ENTRADA`): la lista es quién
///   está adentro (tocar un nombre confirma su salida); el buscador y la
///   cámara buscan en el catálogo completo de contratistas. Elegir a alguien
///   abre su ingreso o, si ya está adentro, su salida (2026-10-26: antes
///   había un modo "Salida" aparte y escanear a quien salía terminaba en "ya
///   tiene un ingreso activo"). Si no aparece, se lo registra ahí mismo.
/// - **Gafete** (`ModoBusqueda.SALIDA_GAFETE`): acepta varios números de
///   gafete separados por coma ("2, 25, 85") — igual que el modo gafete de
///   `SalidaModal.tsx` —
///   y muestra a quién le corresponde cada uno antes de confirmar. Un solo
///   botón registra la salida de todos los que sí tienen ingreso activo de
///   una vez, sin diálogo por persona: es la misma decisión de desktop
///   (pensada para cargar varios gafetes de un tirón), la vista previa con
///   nombres hace las veces de confirmación.
///
/// Se busca siempre con la píldora de abajo ([BuscadorInferior], rediseño
/// 2026-10-25): el campo de arriba se fue y cada modo pone su texto de ayuda
/// y su teclado.
///
/// Todo el estado y las llamadas a [Nucleo] viven en [ActivosViewModel]
/// (ver mobile/android/arquitectura.md) — este archivo sólo dibuja lo que el
/// ViewModel expone y le reporta eventos. Esta función orquesta: delega el
/// selector, los mensajes, la lista y los resultados del buscador (uno por
/// modo, ver [ContenidoListaActivos]/[ResultadosIngreso]/[ResultadosSalida]/
/// [ResultadosGafete] en `ContenidoActivos.kt`) a funciones chicas de una sola responsabilidad cada una, en
/// vez de tener los tres modos mezclados en un único bloque `if`/`else`.
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
    var escanerGafeteSalidaAbierto by remember { mutableStateOf(false) }
    // El buscador de abajo (la píldora de búsqueda), en cualquiera de los
    // tres modos.
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

    // Qué panel se ve ahora (mismo orden de prioridad de siempre: primero el
    // ingreso en curso, luego los escáneres, y si no la lista).
    val panel: PanelActivos = when (val actual = viewModel.seleccionIngreso) {
        is SeleccionIngreso.Formulario -> PanelActivos.Confirmar(actual)
        is SeleccionIngreso.Bloqueada -> PanelActivos.Bloqueado(actual)
        else -> when {
            escanerAbierto -> PanelActivos.EscanerCedula
            escanerGafeteSalidaAbierto -> PanelActivos.EscanerGafete
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
            is PanelActivos.Confirmar -> {
                // Los formularios ocupan toda la pantalla: sin barra de
                // secciones encima (las cámaras la esconden solas).
                OcultarBarraSecciones()
                PantallaConfirmarIngreso(
                    preparacion = mostrado.seleccion.preparacion,
                    error = viewModel.errorIngreso,
                    registrando = viewModel.registrandoIngreso,
                    onRegistrar = viewModel::registrarIngreso,
                    onLimpiarError = viewModel::limpiarErrorIngreso,
                    onCambiar = { viewModel.cancelarSeleccionIngreso() },
                )
            }
            is PanelActivos.Bloqueado -> {
                OcultarBarraSecciones()
                PantallaIngresoBloqueado(
                    preparacion = mostrado.seleccion.preparacion,
                    mensaje = mostrado.seleccion.mensaje,
                    onCambiar = { viewModel.cancelarSeleccionIngreso() },
                )
            }
            PanelActivos.EscanerCedula -> {
                PantallaEscanearCedula(
                    modo = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
                    onDocumentoDetectado = { documento ->
                        if (viewModel.modo != ModoBusqueda.ENTRADA) {
                            viewModel.cambiarModo(ModoBusqueda.ENTRADA)
                        }
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
            PanelActivos.EscanerGafete -> {
                PantallaEscanearCedula(
                    modo = ModoEscaneoDocumento.GAFETE_CONTRATISTA,
                    // Siempre continuo -- pedido explícito del usuario 2026-09-20:
                    // un toggle aparte para decidir "¿sigo escaneando o no?" es un
                    // paso de más, cuando la propia cámara ya tiene un botón para
                    // cerrarla (arriba a la derecha) el día que la persona termine.
                    // Abrir la cámara ya es la acción deliberada de sacar un gafete
                    // -- cada uno que detecta se registra al toque, sin botón de
                    // confirmar, y la cámara se queda armada para el siguiente
                    // hasta que alguien la cierra a mano.
                    continuo = true,
                    // Llena el hueco que dejaba el modo continuo: antes, mientras
                    // la cámara seguía abierta, el mensaje sólo confirmaba que se
                    // LEYÓ el gafete ("Gafete N procesado"), nunca si la salida en
                    // verdad se registró -- un gafete sin ingreso activo quedaba en
                    // silencio, tapado por la propia cámara. Acá se le pasa el
                    // resultado real de la última mutación para que la pantalla de
                    // escaneo lo pinte en el momento (verde/rojo), no sólo el
                    // guardia que mira la lista de abajo después de cerrarla.
                    resultadoUltimoEscaneo = { viewModel.mensaje?.let { it to viewModel.mensajeEsError } },
                    onDocumentoDetectado = { documento ->
                        if (viewModel.modo != ModoBusqueda.SALIDA_GAFETE) {
                            viewModel.cambiarModo(ModoBusqueda.SALIDA_GAFETE)
                        }
                        // Es suspend: el escáner no se rearma hasta que la mutación
                        // terminó. Así nunca entran dos gafetes en paralelo ni se
                        // cancela una salida ya iniciada.
                        viewModel.registrarSalidaPorGafeteEscaneado(documento)
                    },
                    onCerrar = { escanerGafeteSalidaAbierto = false },
                )
            }
            PanelActivos.Lista -> Box(modifier = Modifier.fillMaxSize()) {
            val verificando = viewModel.seleccionIngreso is SeleccionIngreso.Cargando

            // Arriba sólo el modo, los mensajes y la lista: se busca con la
            // píldora de abajo, igual en los tres modos (y en todas las
            // pestañas), al alcance del pulgar.
            Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
                SelectorModoBusqueda(modo = viewModel.modo, onCambiar = { viewModel.cambiarModo(it) })

                MensajesEstado(
                    error = viewModel.error,
                    mensaje = viewModel.mensaje,
                    mensajeEsError = viewModel.mensajeEsError,
                    verificando = verificando,
                )

                Box(modifier = Modifier.padding(top = 4.dp)) {
                    when (viewModel.modo) {
                        ModoBusqueda.ENTRADA -> ContenidoListaActivos(
                            activos = viewModel.activos,
                            cargando = viewModel.cargando,
                            onElegirActivo = { viewModel.elegirSeleccionSalida(it) },
                            sincronizando = sincronizando,
                            onSincronizar = onSincronizar,
                        )
                        ModoBusqueda.SALIDA_GAFETE -> ContenidoModoSalidaGafete()
                    }
                }
            }

            PieFlotante(viewModel.confirmacion, modifier = Modifier.align(Alignment.BottomCenter))
            AccionActivos(
                modo = viewModel.modo,
                onEscanearCedula = { escanerAbierto = true },
                onEscanearGafete = { escanerGafeteSalidaAbierto = true },
            )
            BusquedaPrincipal(ayuda = ayudaBusquedaActivos(viewModel.modo), onAbrir = { buscando = true })

            // El buscador desde abajo, como en iOS, con los modos encima del
            // campo (barra de alcances). Elegir a alguien abre su panel y
            // cierra la búsqueda.
            val cerrarBusqueda = {
                buscando = false
                viewModel.cambiarTexto("")
            }
            BuscadorInferior(
                visible = buscando,
                texto = viewModel.texto,
                onCambiar = { viewModel.cambiarTexto(it) },
                ayuda = ayudaBusquedaActivos(viewModel.modo),
                onCerrar = cerrarBusqueda,
                teclado = tecladoBusquedaActivos(viewModel.modo),
                encabezado = { SelectorModoBusqueda(modo = viewModel.modo, onCambiar = { viewModel.cambiarModo(it) }) },
            ) {
                when (viewModel.modo) {
                    ModoBusqueda.ENTRADA -> Box(modifier = Modifier.padding(horizontal = 16.dp)) {
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
                    ModoBusqueda.SALIDA_GAFETE -> ResultadosGafete(
                        texto = viewModel.texto,
                        coincidencias = viewModel.coincidenciasGafete,
                        enviando = viewModel.enviandoGafetes,
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

            }
        }
    }
}

/// Qué ocupa la pantalla de Activos (ver [PantallaActivos]). Los paneles de
/// cámara no se animan al abrir ni al cerrar.
private sealed interface PanelActivos {
    val conCamara: Boolean get() = false

    data object Lista : PanelActivos
    data class Confirmar(val seleccion: SeleccionIngreso.Formulario) : PanelActivos
    data class Bloqueado(val seleccion: SeleccionIngreso.Bloqueada) : PanelActivos
    data object EscanerCedula : PanelActivos {
        override val conCamara get() = true
    }
    data object EscanerGafete : PanelActivos {
        override val conCamara get() = true
    }
}
