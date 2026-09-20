package com.brisas.controlacceso

import android.util.Log
import android.util.Size
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
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
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

/// Escaneo del Comprobante de Carga de Ruta -- perfil aislado del OCR de
/// identidad (ver LectorComprobanteRuta.kt): reusa la infraestructura de
/// cámara genérica ([iniciarCamara], [analizarCedula], ambas en
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
    val ejecutor = remember { Executors.newSingleThreadExecutor() }
    val recognizer = remember { TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS) }
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
    val detectada = remember { AtomicBoolean(false) }
    val sesionActiva = remember { AtomicBoolean(true) }
    var cameraProvider by remember { mutableStateOf<ProcessCameraProvider?>(null) }
    var vistaPreviaCamara by remember { mutableStateOf<Preview?>(null) }
    var analisisCamara by remember { mutableStateOf<ImageAnalysis?>(null) }
    var trabajoResultado by remember { mutableStateOf<Job?>(null) }
    val ejecutorPrincipal = remember { ContextCompat.getMainExecutor(contexto) }

    DisposableEffect(Unit) {
        sesionActiva.set(true)
        onDispose {
            sesionActiva.set(false)
            detectada.set(true)
            trabajoResultado?.cancel()
            analisisCamara?.clearAnalyzer()
            val casos = listOfNotNull(vistaPreviaCamara, analisisCamara).toTypedArray()
            if (casos.isNotEmpty()) cameraProvider?.unbind(*casos)
            ejecutor.shutdown()
            recognizer.close()
        }
    }

    val colorMarco = when (estado) {
        EstadoEscaneo.CONFIRMADO -> ColorEscaneoConfirmado
        EstadoEscaneo.INVALIDO -> ColorEscaneoInvalido
        EstadoEscaneo.BUSCANDO -> ColorEscaneoBuscando
    }

    Box(modifier = Modifier.fillMaxSize()) {
        AndroidView(
            factory = { ctx ->
                val previewView = PreviewView(ctx).apply { scaleType = PreviewView.ScaleType.FILL_CENTER }
                val analisis = ImageAnalysis.Builder()
                    .setResolutionSelector(
                        ResolutionSelector.Builder()
                            .setResolutionStrategy(
                                // Vuelta a 1280x720 (2026-09-20) -- la
                                // subida a 1920x1080 fue una apuesta sin
                                // confirmar en el dispositivo real y el
                                // usuario reportó que empeoró (parece forzar
                                // un modo de captura distinto). 1280x720 es
                                // la última resolución confirmada como
                                // funcional para este perfil (antes de
                                // cualquiera de los experimentos de
                                // rotación/recorte), y es la misma que usan
                                // las otras 3 pantallas de escaneo.
                                ResolutionStrategy(Size(1280, 720), ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER),
                            )
                            .build(),
                    )
                    .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                    .build()
                    .also { analisisConstruido ->
                        analisisConstruido.setAnalyzer(ejecutor) { imagen ->
                            if (!sesionActiva.get() || detectada.get()) {
                                imagen.close()
                                return@setAnalyzer
                            }
                            val onTexto: (String) -> Unit = { texto ->
                                if (sesionActiva.get()) {
                                    if (BuildConfig.DEBUG) Log.d(TAG_DEBUG_OCR_LECTURA, texto)
                                    val resultado = estabilizador.procesarFrame(texto)
                                    when {
                                        resultado != null -> {
                                            estado = EstadoEscaneo.CONFIRMADO
                                            ultimoMensaje = "Comprobante ${resultado.numeroRuta} confirmado"
                                            if (detectada.compareAndSet(false, true)) {
                                                vibrarConfirmacion(contexto)
                                                trabajoResultado?.cancel()
                                                trabajoResultado = alcance.launch {
                                                    if (sesionActiva.get()) onDetectadoActual(resultado)
                                                }
                                            }
                                        }
                                        // `DetectorTextoNoReconocido` ya tolera
                                        // frames sueltos mal leídos -- ver su
                                        // doc-comment. Si SÍ es un comprobante pero
                                        // todavía no se leyó el "Transporte:"
                                        // (esComprobanteCargaRuta ya dio true), se
                                        // queda en BUSCANDO -- eso no es un
                                        // encuadre inválido, es "sostenga firme".
                                        detectorInvalido.procesarFrame(texto) -> {
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
                            }
                            val onFallo: () -> Unit = {
                                if (sesionActiva.get()) ultimoMensaje = MENSAJE_FALLO_LECTURA_OCR
                            }
                            // Mismo camino simple que usan Vehículo/Ruta y
                            // Carnet KOF -- antes esta pantalla era la única
                            // de las 4 con un camino aparte en debug que
                            // además corría el detector de códigos de barras
                            // en cada frame (sondeo exploratorio del
                            // 2026-09-15 que nunca llegó a una conclusión
                            // útil). Se sacó por completo (2026-09-20): tras
                            // reportarse que el comprobante dejó de
                            // reconocer cualquier cosa, esta pantalla era la
                            // única con ese camino extra sin probar, así que
                            // en vez de seguir adivinando la región de
                            // recorte se unifica con el camino ya
                            // comprobado que sí funciona en las otras 3.
                            analizarCedula(
                                imagen = imagen,
                                recognizer = recognizer,
                                ejecutorPrincipal = ejecutorPrincipal,
                                sesionActiva = sesionActiva,
                                onTexto = onTexto,
                                onFallo = onFallo,
                                // Región propia para el comprobante (2026-09-20,
                                // pedido explícito del usuario) -- ver el
                                // doc-comment de RegionGuiaOcr.COMPROBANTE_RUTA
                                // sobre la escala (comparable a TARJETA_ID,
                                // no el frame casi completo del intento
                                // anterior).
                                region = RegionGuiaOcr.COMPROBANTE_RUTA,
                            )
                        }
                    }
                analisisCamara = analisis
                iniciarCamara(
                    ctx = ctx,
                    previewView = previewView,
                    lifecycleOwner = lifecycleOwner,
                    analisis = analisis,
                    sesionActiva = sesionActiva,
                    onCameraProviderListo = { proveedor, preview ->
                        cameraProvider = proveedor
                        vistaPreviaCamara = preview
                    },
                    onFallo = { mensaje -> if (sesionActiva.get()) ultimoMensaje = mensaje },
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
    }
}

private const val MENSAJE_INICIAL = "Apunte al comprobante de carga de ruta"

