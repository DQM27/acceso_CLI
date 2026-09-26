package com.brisas.controlacceso

import androidx.camera.view.PreviewView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.LocalLifecycleOwner
import kotlinx.coroutines.launch

/// Escaneo del paso 3 del checklist ("Placa o número de unidad", ver
/// `PantallaRutas.kt` y `LectorVehiculoRuta.kt`) -- mismo esqueleto que
/// [PantallaEscanearComprobanteRuta]. Un solo escaneo cubre los dos casos
/// (calcomanía de número de unidad de un camión de flota, o placa de un
/// camión de apoyo): [extraerVehiculo] decide cuál es cuál, esta pantalla
/// no necesita saberlo de antemano.
///
/// `mensajeInicial`/`mensajePermiso` son configurables porque
/// [PantallaProveedores] reusa esta misma pantalla para su OCR de placa
/// (genérico, no específico de rutas -- ver comentario ahí), pero un
/// proveedor nunca trae "número de unidad" (eso es sólo de la flota de
/// rutas) -- mensaje por defecto sin cambios para no alterar Rutas, bug
/// reportado en pruebas reales, 2026-09-17: Proveedores mostraba el
/// mensaje de Rutas tal cual.
@Composable
fun PantallaEscanearVehiculoRuta(
    onVehiculoDetectado: suspend (VehiculoRutaDetectado) -> Unit,
    onCerrar: () -> Unit,
    mensajeInicial: String = MENSAJE_INICIAL_VEHICULO,
    mensajePermiso: String = "Se necesita permiso de cámara para escanear la placa o el número de unidad.",
) {
    EscanerConPermisoCamara(mensajePermiso = mensajePermiso, onCerrar = onCerrar) {
        VistaCamaraVehiculoRuta(
            onVehiculoDetectado = onVehiculoDetectado,
            onCerrar = onCerrar,
            mensajeInicial = mensajeInicial,
        )
    }
}

@Composable
private fun VistaCamaraVehiculoRuta(
    onVehiculoDetectado: suspend (VehiculoRutaDetectado) -> Unit,
    onCerrar: () -> Unit,
    mensajeInicial: String,
) {
    val contexto = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val alcance = rememberCoroutineScope()
    val onDetectadoActual by rememberUpdatedState(onVehiculoDetectado)
    var ultimoMensaje by remember { mutableStateOf(mensajeInicial) }
    var estado by remember { mutableStateOf(EstadoEscaneo.BUSCANDO) }
    val estabilizador = remember {
        EstabilizadorPorRepeticion(extraer = ::extraerVehiculo, clave = { "${it.tipo}:${it.valor}" })
    }
    val detectorInvalido = remember { DetectorTextoNoReconocido(esTipoEsperado = { extraerVehiculo(it) != null }) }
    // MV-10 (auditoría 2026-09-24): ver EstadoCamaraOcr.kt.
    val camara = rememberEstadoCamaraOcr(contexto)

    val colorMarco = when (estado) {
        EstadoEscaneo.CONFIRMADO -> ColorEscaneoConfirmado
        EstadoEscaneo.INVALIDO -> ColorEscaneoInvalido
        EstadoEscaneo.BUSCANDO -> ColorEscaneoBuscando
    }

    Box(modifier = Modifier.fillMaxSize()) {
        AndroidView(
            factory = { ctx ->
                val previewView = PreviewView(ctx).apply { scaleType = PreviewView.ScaleType.FILL_CENTER }
                val analisis = construirAnalizadorOcr(
                    ejecutorAnalisis = camara.ejecutor,
                    detectada = camara.detectada,
                    sesionActiva = camara.sesionActiva,
                ) { imagen ->
                    analizarCedula(
                        imagen = imagen,
                        recognizer = camara.recognizer,
                        ejecutorPrincipal = camara.ejecutorPrincipal,
                        sesionActiva = camara.sesionActiva,
                        onTexto = { texto ->
                            if (camara.sesionActiva.get()) {
                                val resultado = estabilizador.procesarFrame(texto)
                                when {
                                    resultado != null -> {
                                        estado = EstadoEscaneo.CONFIRMADO
                                        ultimoMensaje = "${resultado.valor} confirmado"
                                        if (camara.detectada.compareAndSet(false, true)) {
                                            vibrarConfirmacion(contexto)
                                            reproducirSonidoConfirmacion()
                                            camara.trabajoResultado?.cancel()
                                            camara.trabajoResultado = alcance.launch {
                                                if (camara.sesionActiva.get()) onDetectadoActual(resultado)
                                            }
                                        }
                                    }
                                    // Sin clasificador aparte acá (a
                                    // diferencia de Comprobante/Carnet KOF):
                                    // placa/número de unidad es un dato
                                    // atómico, `extraerVehiculo` ya decide
                                    // todo en un solo paso -- que falle es
                                    // en sí mismo la señal de "esto no es
                                    // una placa ni un número de unidad".
                                    // Ver `DetectorTextoNoReconocido` sobre
                                    // por qué esto tolera frames sueltos.
                                    detectorInvalido.procesarFrame(texto) -> {
                                        if (estado != EstadoEscaneo.INVALIDO) vibrarError(contexto)
                                        estado = EstadoEscaneo.INVALIDO
                                        ultimoMensaje = "No se reconoce como placa ni número de unidad"
                                    }
                                    else -> {
                                        estado = EstadoEscaneo.BUSCANDO
                                        ultimoMensaje = mensajeInicial
                                    }
                                }
                            }
                        },
                        onFallo = {
                            if (camara.sesionActiva.get()) ultimoMensaje = MENSAJE_FALLO_LECTURA_OCR
                        },
                    )
                }
                camara.analisisCamara = analisis
                iniciarCamara(
                    ctx = ctx,
                    previewView = previewView,
                    lifecycleOwner = lifecycleOwner,
                    analisis = analisis,
                    sesionActiva = camara.sesionActiva,
                    onCameraProviderListo = { proveedor, preview ->
                        camara.cameraProvider = proveedor
                        camara.vistaPreviaCamara = preview
                    },
                    onFallo = { mensaje -> if (camara.sesionActiva.get()) ultimoMensaje = mensaje },
                )
                previewView
            },
            modifier = Modifier.fillMaxSize(),
        )
        MarcoGuiaCedula(color = colorMarco, estado = estado, modifier = Modifier.fillMaxSize())
        Text(
            ultimoMensaje,
            color = Color.White,
            style = MaterialTheme.typography.bodyLarge,
            modifier = Modifier
                .fillMaxWidth()
                .align(Alignment.TopCenter)
                .padding(top = 16.dp, start = 16.dp, end = 72.dp)
                .background(Color.Black.copy(alpha = 0.78f), FormaCampoBrisas)
                .padding(12.dp),
        )
        // Mismo botón compartido que las otras 3 pantallas de escaneo
        // (`ControlesBrisas.kt`) -- antes era el texto "Cancelar" (hallazgo
        // 2026-09-19).
        BotonCerrarCamara(onClick = onCerrar, modifier = Modifier.align(Alignment.TopEnd).padding(16.dp))
    }
}

private const val MENSAJE_INICIAL_VEHICULO = "Apunte a la placa o al número de unidad"
