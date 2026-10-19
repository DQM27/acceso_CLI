package com.brisas.controlacceso

import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.Nucleo
/// Una sola vista para el ciclo completo — entrada, permanencia y salida —,
/// igual que `Activos.tsx` en desktop (ahí "+Nuevo"/"Salida" abren modales
/// sobre la misma grilla de activos; ver docs/plan-app-movil.md, addendum
/// 2026-09-01). Acá no hay una pestaña "Buscar" aparte: el mismo campo de
/// texto cambia de sentido según el modo elegido en el selector de arriba
/// — mismo espíritu que el checkbox "Por gafete" de `SalidaModal.tsx` (un
/// solo campo, la interpretación cambia), llevado a un selector de tres
/// porque acá hace falta distinguir tres búsquedas, no dos:
///
/// - **Ingreso** (por defecto, `ModoBusqueda.ENTRADA`): vacío lista quién
///   está adentro (tocar un nombre confirma su salida); con texto busca en
///   el catálogo completo de contratistas (antes pestaña "Buscar" aparte)
///   para arrancar el flujo de confirmar entrada.
/// - **Salida** (`ModoBusqueda.SALIDA_NOMBRE`): filtra la lista de activos
///   por cédula/nombre — tocar un resultado abre el mismo diálogo de
///   confirmar salida de siempre. Vacío no trae nada (es un buscador, no
///   una lista para recorrer — para eso ya está la pestaña Ingreso).
/// - **Gafete** (`ModoBusqueda.SALIDA_GAFETE`): acepta varios números de
///   gafete separados por coma ("2, 25, 85") — igual que el modo gafete de
///   `SalidaModal.tsx` —
///   y muestra a quién le corresponde cada uno antes de confirmar. Un solo
///   botón registra la salida de todos los que sí tienen ingreso activo de
///   una vez, sin diálogo por persona: es la misma decisión de desktop
///   (pensada para cargar varios gafetes de un tirón), la vista previa con
///   nombres hace las veces de confirmación.
///
/// Todo el estado y las llamadas a [Nucleo] viven en [ActivosViewModel]
/// (ver mobile/android/arquitectura.md) — este archivo sólo dibuja lo que el
/// ViewModel expone y le reporta eventos. Esta función orquesta: delega el
/// selector, el campo, los mensajes y el contenido (uno por modo, ver
/// [ContenidoModoEntrada]/[ContenidoModoSalidaNombre]/[ContenidoModoSalidaGafete]
/// más abajo) a funciones chicas de una sola responsabilidad cada una, en
/// vez de tener los tres modos mezclados en un único bloque `if`/`else`.
@Composable
fun PantallaActivos(
    nucleo: Nucleo,
    refrescarNube: Int = 0,
) {
    RegistrarPantalla("activos")
    val viewModel: ActivosViewModel =
        viewModel(factory = ActivosViewModel.factory(nucleo))
    var escanerAbierto by remember { mutableStateOf(false) }
    var escanerGafeteSalidaAbierto by remember { mutableStateOf(false) }
    LaunchedEffect(refrescarNube) {
        if (refrescarNube > 0) {
            viewModel.refrescar()
        }
    }

    // "Salida de X · Deshacer" mientras una salida deslizada espera (ver
    // `ActivosViewModel.deslizarSalida`). El aviso dura lo que dura la espera:
    // cuando la salida se registra, `salidaPendiente` vuelve a null, este efecto
    // se cancela y el aviso desaparece solo.
    val avisos = remember { SnackbarHostState() }
    val salidaPendiente = viewModel.salidaPendiente
    LaunchedEffect(salidaPendiente?.clave) {
        if (salidaPendiente != null) {
            val resultado = avisos.showSnackbar(
                message = "Salida de ${salidaPendiente.contratistaNombre}",
                actionLabel = "Deshacer",
                duration = SnackbarDuration.Indefinite,
            )
            if (resultado == SnackbarResult.ActionPerformed) viewModel.deshacerSalida()
        }
    }
    // Si la app pasa a segundo plano o se sale de la pantalla, la salida que
    // esperaba se registra ya: no queda nada en el aire.
    LifecycleEventEffect(Lifecycle.Event.ON_STOP) { viewModel.confirmarSalidaPendiente() }
    DisposableEffect(viewModel) { onDispose { viewModel.confirmarSalidaPendiente() } }

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
                        escanerAbierto = false
                        if (viewModel.modo != ModoBusqueda.ENTRADA) {
                            viewModel.cambiarModo(ModoBusqueda.ENTRADA)
                        }
                        viewModel.usarDocumentoEscaneadoIngreso(documento)
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

            Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
                SelectorModoBusqueda(modo = viewModel.modo, onCambiar = { viewModel.cambiarModo(it) })

                CampoBusquedaActivos(
                    modo = viewModel.modo,
                    texto = viewModel.texto,
                    onCambiarTexto = { viewModel.cambiarTexto(it) },
                    onEscanearCedula = { escanerAbierto = true },
                    onEscanearGafete = { escanerGafeteSalidaAbierto = true },
                )

                MensajesEstado(
                    error = viewModel.error,
                    mensaje = viewModel.mensaje,
                    mensajeEsError = viewModel.mensajeEsError,
                    verificando = verificando,
                )

                when (viewModel.modo) {
                    ModoBusqueda.ENTRADA -> ContenidoModoEntrada(
                        texto = viewModel.texto,
                        activos = viewModel.activosVisibles,
                        resultadosBusqueda = viewModel.resultadosBusqueda,
                        verificando = verificando,
                        cargando = viewModel.cargando,
                        onElegirActivo = { viewModel.elegirSeleccionSalida(it) },
                        onElegirContratista = { viewModel.elegir(it) },
                        onDeslizarSalida = viewModel::deslizarSalida,
                    )
                    ModoBusqueda.SALIDA_NOMBRE -> ContenidoModoSalidaNombre(
                        activos = viewModel.activosVisibles,
                        onElegirActivo = { viewModel.elegirSeleccionSalida(it) },
                        onDeslizarSalida = viewModel::deslizarSalida,
                    )
                    ModoBusqueda.SALIDA_GAFETE -> ContenidoModoSalidaGafete(
                        texto = viewModel.texto,
                        coincidencias = viewModel.coincidenciasGafete,
                        enviando = viewModel.enviandoGafetes,
                        onRegistrarSalidaGafetes = { viewModel.registrarSalidaPorGafetes() },
                    )
                }
            }

            DialogoConfirmarSalida(
                fila = viewModel.seleccionSalida,
                onDismiss = { viewModel.elegirSeleccionSalida(null) },
                onConfirmar = { viewModel.confirmarSalida(it) },
            )

            SnackbarHost(
                hostState = avisos,
                modifier = Modifier.align(Alignment.BottomCenter).padding(bottom = 8.dp),
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
