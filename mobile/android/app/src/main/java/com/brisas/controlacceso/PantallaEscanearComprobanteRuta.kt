package com.brisas.controlacceso

import android.util.Log
import androidx.camera.view.PreviewView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
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

/// Escaneo del Comprobante de Carga de Ruta -- perfil aislado del OCR de
/// identidad (ver LectorComprobanteRuta.kt): reusa la infraestructura de
/// cámara genérica ([iniciarCamara], [analizarFrameOcr], ambas en
/// `PantallaEscanearCedula.kt` -- no conocen nada de cédulas, sólo entregan
/// texto crudo de ML Kit), pero con su propia estabilización y su propio
/// resultado ([ComprobanteRutaDetectado]). Un solo escaneo carga ruta,
/// sub-número (tipo) y documento a la vez porque el comprobante real trae
/// los tres juntos en la misma línea impresa.
///
/// Resolución de análisis: 1280x720, igual que las otras 3 pantallas.
/// Hubo acá una subida a 1920x1080 (2026-09-20, hipótesis de que el texto
/// impreso más chico de una hoja completa necesitaba más píxeles por
/// carácter que ML Kit) -- se revirtió: probada contra el dispositivo real
/// empeoró la lectura en vez de mejorarla, así que se vuelve a 1280x720,
/// la última resolución confirmada como funcional para este perfil, de
/// antes de cualquiera de los experimentos de rotación/recorte.
@Composable
fun PantallaEscanearComprobanteRuta(
    onComprobanteDetectado: suspend (ComprobanteRutaDetectado) -> Unit,
    onCerrar: () -> Unit,
) {
    EscanerConPermisoCamara(
        mensajePermiso = "Se necesita permiso de cámara para escanear el comprobante.",
        onCerrar = onCerrar,
    ) {
        VistaCamaraComprobanteRuta(onComprobanteDetectado = onComprobanteDetectado, onCerrar = onCerrar)
    }
}

@Composable
private fun VistaCamaraComprobanteRuta(
    onComprobanteDetectado: suspend (ComprobanteRutaDetectado) -> Unit,
    onCerrar: () -> Unit,
) {
    val contexto = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val alcance = rememberCoroutineScope()
    val onDetectadoActual by rememberUpdatedState(onComprobanteDetectado)
    var ultimoMensaje by remember { mutableStateOf(MENSAJE_INICIAL) }
    var estado by remember { mutableStateOf(EstadoEscaneo.BUSCANDO) }
    // Log, no overlay en pantalla (pedido explícito del usuario 2026-09-20:
    // se veía mal encima de la cámara) -- `adb logcat -s
    // $TAG_DEBUG_OCR_LECTURA` en un build debug sigue alcanzando para
    // ajustar los regex de LectorComprobanteRuta.kt sin adivinar a ciegas.
    val estabilizador = remember {
        EstabilizadorPorRepeticion(
            extraer = ::extraerComprobanteRuta,
            clave = { "${it.numeroRuta}:${it.subNumero}:${it.numeroDocumento}" },
        )
    }
    val detectorInvalido = remember { DetectorTextoNoReconocido(esTipoEsperado = ::esComprobanteCargaRuta) }
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
                // Resolución: la misma de las otras 3 pantallas, fijada una
                // sola vez en `construirAnalizadorOcr` (1920x1080, ver ahí).
                val analisis = construirAnalizadorOcr(
                    ejecutorAnalisis = camara.ejecutor,
                    detectada = camara.detectada,
                    sesionActiva = camara.sesionActiva,
                ) { imagen ->
                    analizarFrameOcr(
                        imagen = imagen,
                        camara = camara,
                        region = RegionGuiaOcr.COMPROBANTE_RUTA,
                        onLectura = { lectura ->
                            // Hilo del analizador: extraer y votar acá, a la
                            // pantalla sólo se publica el resultado.
                            val texto = lectura.texto
                            if (BuildConfig.DEBUG) Log.d(TAG_DEBUG_OCR_LECTURA, texto)
                            val resultado = estabilizador.procesarFrame(texto, lectura.peso)
                            // Sólo si no hubo resultado, como antes: un
                            // frame que confirma no cuenta como inválido.
                            val invalido = resultado == null && detectorInvalido.procesarFrame(texto)
                            camara.enPrincipal {
                                when {
                                    resultado != null -> {
                                        estado = EstadoEscaneo.CONFIRMADO
                                        ultimoMensaje = "Comprobante ${resultado.numeroRuta} confirmado"
                                        if (camara.detectada.compareAndSet(false, true)) {
                                            camara.metricas.registrarConfirmacion()
                                            vibrarConfirmacion(contexto)
                                            camara.trabajoResultado?.cancel()
                                            camara.trabajoResultado = alcance.launch {
                                                if (camara.sesionActiva.get()) onDetectadoActual(resultado)
                                            }
                                        }
                                    }
                                    // `DetectorTextoNoReconocido` tolera
                                    // frames sueltos mal leídos -- ver su
                                    // doc-comment.
                                    invalido -> {
                                        if (estado != EstadoEscaneo.INVALIDO) vibrarError(contexto)
                                        estado = EstadoEscaneo.INVALIDO
                                        ultimoMensaje = "Documento no reconocido"
                                    }
                                    else -> {
                                        estado = EstadoEscaneo.BUSCANDO
                                        ultimoMensaje = MENSAJE_INICIAL
                                    }
                                }
                            }
                        },
                        onFallo = { camara.enPrincipal { ultimoMensaje = MENSAJE_FALLO_LECTURA_OCR } },
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
                    onCamaraLista = { camara.camaraFisica = it },
                    onFallo = { mensaje -> if (camara.sesionActiva.get()) ultimoMensaje = mensaje },
                )
                previewView
            },
            modifier = Modifier.fillMaxSize(),
        )
        MarcoGuiaCedula(
            color = colorMarco,
            estado = estado,
            modifier = Modifier.fillMaxSize(),
            region = RegionGuiaOcr.COMPROBANTE_RUTA,
        )
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
        if (camara.tieneLinterna) {
            BotonLinterna(
                encendida = camara.linternaEncendida,
                onClick = camara::alternarLinterna,
                modifier = Modifier.align(Alignment.TopEnd).padding(top = 72.dp, end = 16.dp),
            )
        }
    }
}

private const val MENSAJE_INICIAL = "Apunte al comprobante de carga de ruta"

