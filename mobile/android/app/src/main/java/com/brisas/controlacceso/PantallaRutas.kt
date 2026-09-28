package com.brisas.controlacceso

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.Ruta
import uniffi.control_acceso_mobile.SalidaRutaActivaResumen
import uniffi.control_acceso_mobile.SolicitudSalidaRuta
import uniffi.control_acceso_mobile.VehiculoRuta
/// Checklist real del módulo de rutas -- ver
/// `docs/planes-implementados/plan-control-rutas.md`. Conectado al núcleo
/// real desde 2026-09-15 (antes era un mock en memoria, ver
/// [RutasViewModel]). Orden de la tarjeta sin cambios respecto al primer
/// corte (encargado → documento de ruta → vehículo) -- fue el desktop el
/// que se reordenó para igualar este orden, no al revés. Tres cambios de
/// fondo en esta vuelta, los tres a pedido explícito del usuario
/// (2026-09-15): (1) "Encargado" ahora es un buscador real por nombre o
/// código (mismo criterio que el buscador de contratistas), ya no
/// descarta el código en silencio; (2) "N.º de ruta" ahora es un
/// buscador BLOQUEANTE contra el catálogo -- el paso no se da por
/// completo sin elegir una [Ruta] real; (3) se quitó el botón
/// "Documento (H)" (agregar sub-documentos H2-H4 a una salida ya
/// registrada) -- no tiene contraparte en el núcleo, así que se retira
/// del checklist por ahora en vez de dejarlo simulado.
@Composable
fun PantallaRutas(nucleo: Nucleo) {
    RegistrarPantalla("rutas")
    val viewModel: RutasViewModel = viewModel(factory = RutasViewModel.factory(nucleo))

    var subNumeroTexto by remember { mutableStateOf("1") }
    var numeroDocumento by remember { mutableStateOf("") }
    var fechaDocumentoTexto by remember { mutableStateOf(fechaHoyTextoRuta()) }
    var tieneCorreo by remember { mutableStateOf(false) }

    var salidaParaConfirmarRetorno by remember { mutableStateOf<SalidaRutaActivaResumen?>(null) }
    var escanerRutaAbierto by remember { mutableStateOf(false) }
    var escanerCarnetKofAbierto by remember { mutableStateOf(false) }
    var escanerVehiculoAbierto by remember { mutableStateOf(false) }

    if (escanerRutaAbierto) {
        PantallaEscanearComprobanteRuta(
            onComprobanteDetectado = { comprobante ->
                escanerRutaAbierto = false
                extraerDigitosRuta(comprobante.numeroRuta)?.let(viewModel::usarNumeroRutaEscaneado)
                subNumeroTexto = comprobante.subNumero.toString()
                numeroDocumento = comprobante.numeroDocumento
            },
            onCerrar = { escanerRutaAbierto = false },
        )
        return
    }

    if (escanerCarnetKofAbierto) {
        PantallaEscanearCarnetKof(
            onCarnetDetectado = { carnet ->
                escanerCarnetKofAbierto = false
                // El código de empleado identifica sin ambigüedad -- se
                // prefiere sobre el nombre cuando el carnet trae los dos
                // (mismo criterio que el buscador de contratistas, que
                // resuelve por cédula antes que por nombre cuando ambos
                // vienen del OCR).
                val texto = carnet.codigoEmpleado ?: carnet.nombre
                if (texto != null) viewModel.usarEncargadoEscaneado(texto)
            },
            onCerrar = { escanerCarnetKofAbierto = false },
        )
        return
    }

    if (escanerVehiculoAbierto) {
        PantallaEscanearVehiculoRuta(
            onVehiculoDetectado = { detectado ->
                escanerVehiculoAbierto = false
                // Un solo buscador contra el catálogo ahora -- da igual si
                // el OCR leyó placa o número de unidad, los dos buscan
                // contra el mismo [VehiculoRuta] (ver
                // [RutasViewModel.usarVehiculoEscaneado]).
                viewModel.usarVehiculoEscaneado(detectado.valor)
            },
            onCerrar = { escanerVehiculoAbierto = false },
        )
        return
    }

    val rutaSeleccionada = viewModel.rutaSeleccionada
    val encargadoSeleccionado = viewModel.encargadoSeleccionado
    val paso1Completo = encargadoSeleccionado != null
    val paso2Completo = rutaSeleccionada != null && numeroDocumento.isNotBlank() && fechaDocumentoTexto.isNotBlank()
    val vehiculoSeleccionado = viewModel.vehiculoSeleccionado
    val paso3Completo = vehiculoSeleccionado != null
    // Ya no bloquea la confirmación (pedido explícito del usuario
    // 2026-09-20: "elimina la limitación de la fecha") -- `fechaVencida`
    // se queda sólo como aviso visual en `PasoDocumentoRuta`, y
    // `tieneCorreo` sigue viajando al backend (`tieneCorreoAutorizacion`)
    // aunque ya no exista nada que desbloquear con él.
    val fechaVencida = fechaDocumentoTexto.isNotBlank() && fechaDocumentoTexto != fechaHoyTextoRuta()
    val puedeConfirmar =
        paso1Completo && paso2Completo && paso3Completo && !viewModel.registrando

    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
        // Scroll propio para este bloque -- sin esto, el teclado tapaba el
        // input de "Placa o número de unidad" (paso 3) sin forma de
        // deslizar hasta él (reportado 2026-09-20). No se puede envolver
        // TODA la pantalla en un solo `verticalScroll` como en
        // [PantallaProveedores] porque abajo hay un `LazyColumn` ("Rutas
        // activas") -- los dos no combinan en un mismo eje de scroll.
        // `imePadding()` deja que el teclado empuje este bloque hacia
        // arriba y el campo enfocado se desplace por encima de él.
        val scrollStateFormulario = rememberScrollState()
        val scopeFormulario = rememberCoroutineScope()
        Column(
            modifier = Modifier.verticalScroll(scrollStateFormulario).imePadding(),
        ) {
        Text(
            "Registrar salida",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(bottom = 10.dp),
        )

        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
            PasoEncargado(
                completado = paso1Completo,
                texto = viewModel.textoEncargado,
                onCambiarTexto = viewModel::cambiarTextoEncargado,
                resultados = viewModel.resultadosEncargado,
                onElegir = viewModel::elegirEncargado,
                sinCoincidencias =
                    viewModel.textoEncargado.isNotBlank() &&
                        encargadoSeleccionado == null &&
                        viewModel.resultadosEncargado.isEmpty(),
                onEscanear = { escanerCarnetKofAbierto = true },
            )
            PasoDocumentoRuta(
                completado = paso2Completo,
                textoRuta = viewModel.textoRuta,
                onCambiarTextoRuta = viewModel::cambiarTextoRuta,
                resultadosRuta = viewModel.resultadosRuta,
                onElegirRuta = viewModel::elegirRuta,
                rutaSinCoincidencias =
                    viewModel.textoRuta.isNotBlank() && rutaSeleccionada == null && viewModel.resultadosRuta.isEmpty(),
                etiquetaTipo = etiquetaSubNumero(subNumeroTexto),
                onTocarTipo = {
                    subNumeroTexto = ((subNumeroTexto.toIntOrNull() ?: 1) % 4 + 1).toString()
                },
                numeroDocumento = numeroDocumento,
                onCambiarNumeroDocumento = { numeroDocumento = it },
                onEscanear = { escanerRutaAbierto = true },
                fechaVencida = fechaVencida,
                tieneCorreo = tieneCorreo,
                onCambiarTieneCorreo = { tieneCorreo = it },
            )
            PasoVehiculo(
                completado = paso3Completo,
                texto = viewModel.textoVehiculo,
                onCambiarTexto = viewModel::cambiarTextoVehiculo,
                resultados = viewModel.resultadosVehiculo,
                onElegir = viewModel::elegirVehiculo,
                sinCoincidencias =
                    viewModel.textoVehiculo.isNotBlank() &&
                        vehiculoSeleccionado == null &&
                        viewModel.resultadosVehiculo.isEmpty(),
                onEscanear = { escanerVehiculoAbierto = true },
                // Al enfocar el último input, lleva el scroll hasta el
                // fondo -- ahí vive "Confirmar salida", último elemento de
                // esta misma Column. El foco por sí solo sólo garantiza que
                // el campo entre en pantalla, no el botón de abajo (pedido
                // explícito del usuario 2026-09-20). Un solo
                // `animateScrollTo` no alcanza: el teclado tarda ~250ms en
                // animarse y el `imePadding()` va agrandando la Column
                // cuadro a cuadro, así que `maxValue` todavía no refleja el
                // alto final en el instante del foco -- se repite mientras
                // dura esa animación para perseguir el nuevo fondo.
                onEnfocado = {
                    scopeFormulario.launch {
                        repeat(15) {
                            scrollStateFormulario.animateScrollTo(scrollStateFormulario.maxValue)
                            delay(30)
                        }
                    }
                },
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
                val ruta = rutaSeleccionada ?: return@BotonBrisas
                val vehiculo = vehiculoSeleccionado ?: return@BotonBrisas
                val encargado = encargadoSeleccionado ?: return@BotonBrisas
                viewModel.registrarSalida(
                    SolicitudSalidaRuta(
                        vehiculoPlaca = vehiculo.placa,
                        vehiculoNumeroUnidad = vehiculo.numeroUnidad,
                        encargadoNombre = encargado.nombre,
                        encargadoCodigoEmpleado = encargado.codigoEmpleado,
                        numeroRuta = ruta.numero,
                        subNumero = (subNumeroTexto.toLongOrNull() ?: 1L),
                        numeroDocumento = numeroDocumento,
                        fechaDocumento = textoDDMMYYYYaIso(fechaDocumentoTexto),
                        tieneCorreoAutorizacion = tieneCorreo,
                    ),
                    onExito = {
                        subNumeroTexto = "1"
                        numeroDocumento = ""
                        fechaDocumentoTexto = fechaHoyTextoRuta()
                        tieneCorreo = false
                    },
                )
            },
            enabled = puedeConfirmar,
            modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
        ) {
            Text("Confirmar salida")
        }
        }

        Text(
            "Rutas activas",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(top = 16.dp, bottom = 6.dp),
        )

        ListaConDesvanecido {
            LazyColumn(
                contentPadding = PaddingValues(top = 5.dp),
                verticalArrangement = Arrangement.spacedBy(5.dp),
            ) {
                items(viewModel.activas, key = { it.id }) { salida ->
                    FilaSalidaRuta(
                        salida = salida,
                        onConfirmarRetorno = { salidaParaConfirmarRetorno = salida },
                    )
                }
            }
        }
    }

    DialogoConfirmarRetornoRuta(
        salida = salidaParaConfirmarRetorno,
        onDismiss = { salidaParaConfirmarRetorno = null },
        onConfirmar = {
            viewModel.registrarRetorno(it)
            salidaParaConfirmarRetorno = null
        },
    )
}
