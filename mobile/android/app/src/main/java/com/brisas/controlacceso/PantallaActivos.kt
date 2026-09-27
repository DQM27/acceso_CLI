package com.brisas.controlacceso

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
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
    secretoStore: SecretoDispositivoStore,
    refrescarNube: Int = 0,
) {
    val viewModel: ActivosViewModel =
        viewModel(factory = ActivosViewModel.factory(nucleo, secretoStore))
    var escanerAbierto by remember { mutableStateOf(false) }
    var escanerGafeteSalidaAbierto by remember { mutableStateOf(false) }
    LaunchedEffect(refrescarNube) {
        if (refrescarNube > 0) {
            viewModel.refrescar()
        }
    }

    when (val actual = viewModel.seleccionIngreso) {
        is SeleccionIngreso.Formulario -> {
            PantallaConfirmarIngreso(
                preparacion = actual.preparacion,
                error = viewModel.errorIngreso,
                registrando = viewModel.registrandoIngreso,
                onRegistrar = viewModel::registrarIngreso,
                onLimpiarError = viewModel::limpiarErrorIngreso,
                onCambiar = { viewModel.cancelarSeleccionIngreso() },
            )
            return
        }
        is SeleccionIngreso.Bloqueada -> {
            PantallaIngresoBloqueado(
                preparacion = actual.preparacion,
                mensaje = actual.mensaje,
                onCambiar = { viewModel.cancelarSeleccionIngreso() },
            )
            return
        }
        else -> Unit
    }

    if (escanerAbierto) {
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
        return
    }

    if (escanerGafeteSalidaAbierto) {
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
        return
    }

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
                activos = viewModel.activos,
                resultadosBusqueda = viewModel.resultadosBusqueda,
                verificando = verificando,
                cargando = viewModel.cargando,
                onElegirActivo = { viewModel.elegirSeleccionSalida(it) },
                onElegirContratista = { viewModel.elegir(it) },
            )
            ModoBusqueda.SALIDA_NOMBRE -> ContenidoModoSalidaNombre(
                activos = viewModel.activos,
                onElegirActivo = { viewModel.elegirSeleccionSalida(it) },
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
}
