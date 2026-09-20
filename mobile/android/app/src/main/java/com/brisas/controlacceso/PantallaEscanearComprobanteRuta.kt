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
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.common.InputImage
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
    val haptica = LocalHapticFeedback.current
    val ejecutor = remember { Executors.newSingleThreadExecutor() }
    val recognizer = remember { TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS) }
    // Sólo se crea en debug (ver uso más abajo) -- sondeo exploratorio del
    // código de barras del comprobante: todavía no sabemos qué dato trae
    // (¿el mismo "Transporte"? ¿otra cosa?), así que por ahora sólo se lee
    // y se muestra, no reemplaza ni complementa la extracción por texto
    // hasta confirmar qué dice contra un comprobante real.
    val barcodeScanner = remember { if (BuildConfig.DEBUG) BarcodeScanning.getClient() else null }
    var ultimoMensaje by remember { mutableStateOf(MENSAJE_INICIAL) }
    var estado by remember { mutableStateOf(EstadoEscaneo.BUSCANDO) }
    // Log, no overlay en pantalla (pedido explícito del usuario 2026-09-20:
    // se veía mal encima de la cámara) -- `adb logcat -s
    // $TAG_DEBUG_OCR_LECTURA` en un build debug sigue alcanzando para
    // ajustar los regex de LectorComprobanteRuta.kt sin adivinar a ciegas.
    // Comparación exploratoria (2026-09-15): el usuario probó el sondeo del
    // código de barras contra el papel real para ver si "dice lo mismo" que
    // "Transporte:" -- se comparan los valores ya parseados y el resultado
    // también va a Log, no a un Toast/overlay.
    var numeroDocumentoTextoDebug by remember { mutableStateOf<String?>(null) }
    var barcodeValorDebug by remember { mutableStateOf<String?>(null) }
    var yaAvisoComparacion by remember { mutableStateOf(false) }
    val estabilizador = remember {
        EstabilizadorPorRepeticion(
            extraer = ::extraerComprobanteRuta,
            clave = { "${it.numeroRuta}:${it.subNumero}:${it.numeroDocumento}" },
        )
    }
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
            barcodeScanner?.close()
        }
    }

    val colorMarco = if (estado == EstadoEscaneo.CONFIRMADO) ColorEscaneoConfirmado else ColorEscaneoBuscando

    Box(modifier = Modifier.fillMaxSize()) {
        AndroidView(
            factory = { ctx ->
                val previewView = PreviewView(ctx).apply { scaleType = PreviewView.ScaleType.FILL_CENTER }
                val analisis = ImageAnalysis.Builder()
                    .setResolutionSelector(
                        ResolutionSelector.Builder()
                            .setResolutionStrategy(
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
                                    if (BuildConfig.DEBUG) {
                                        Log.d(TAG_DEBUG_OCR_LECTURA, texto)
                                        numeroDocumentoTextoDebug = extraerComprobanteRuta(texto)?.numeroDocumento
                                    }
                                    val resultado = estabilizador.procesarFrame(texto)
                                    if (resultado != null) {
                                        estado = EstadoEscaneo.CONFIRMADO
                                        ultimoMensaje = "Comprobante ${resultado.numeroRuta} confirmado"
                                        if (detectada.compareAndSet(false, true)) {
                                            haptica.performHapticFeedback(HapticFeedbackType.Confirm)
                                            trabajoResultado?.cancel()
                                            trabajoResultado = alcance.launch {
                                                if (sesionActiva.get()) onDetectadoActual(resultado)
                                            }
                                        }
                                    } else {
                                        estado = EstadoEscaneo.BUSCANDO
                                        ultimoMensaje = MENSAJE_INICIAL
                                    }
                                }
                            }
                            val onFallo: () -> Unit = {
                                if (sesionActiva.get()) ultimoMensaje = MENSAJE_FALLO_LECTURA_OCR
                            }
                            val scannerBarcodeActual = barcodeScanner
                            if (scannerBarcodeActual != null) {
                                // Sólo el camino debug corre los dos detectores por
                                // frame (texto + barcode) -- ver comentario de
                                // `barcodeScanner` más arriba. El camino de
                                // producción (`else`) sigue usando únicamente
                                // `analizarCedula`, sin el costo extra.
                                analizarComprobanteConBarcodeDebug(
                                    imagen = imagen,
                                    recognizer = recognizer,
                                    barcodeScanner = scannerBarcodeActual,
                                    ejecutorPrincipal = ejecutorPrincipal,
                                    sesionActiva = sesionActiva,
                                    onTexto = onTexto,
                                    onBarcodes = { codigos ->
                                        if (sesionActiva.get() && codigos.isNotEmpty()) {
                                            Log.d(
                                                TAG_DEBUG_OCR_LECTURA,
                                                "barcode: " + codigos.joinToString("; ") { "${it.rawValue} (formato ${it.format})" },
                                            )
                                            barcodeValorDebug = codigos.firstOrNull()?.rawValue?.trim()
                                        }
                                    },
                                    onFallo = onFallo,
                                )
                            } else {
                                analizarCedula(
                                    imagen = imagen,
                                    recognizer = recognizer,
                                    ejecutorPrincipal = ejecutorPrincipal,
                                    sesionActiva = sesionActiva,
                                    onTexto = onTexto,
                                    onFallo = onFallo,
                                )
                            }
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
        if (BuildConfig.DEBUG) {
            val docTexto = numeroDocumentoTextoDebug
            val docBarcode = barcodeValorDebug
            if (docTexto != null && docBarcode != null) {
                LaunchedEffect(docTexto, docBarcode) {
                    if (!yaAvisoComparacion) {
                        yaAvisoComparacion = true
                        val coincide = docTexto == docBarcode
                        Log.d(
                            TAG_DEBUG_OCR_LECTURA,
                            if (coincide) {
                                "barcode coincide con Transporte ($docTexto)"
                            } else {
                                "barcode NO coincide -- texto=$docTexto barcode=$docBarcode"
                            },
                        )
                    }
                }
            }
        }
    }
}

/// Variante DEBUG-only de [analizarCedula] que además corre el detector de
/// códigos de barras de ML Kit sobre el mismo frame -- exploratorio: el
/// usuario pidió ver qué dato trae el código de barras del comprobante de
/// ruta (visible justo debajo de "Transporte:" en las 4 fotos reales) para
/// decidir si conviene usarlo en vez de (o además de) la extracción por
/// texto. Se ejecuta el reconocedor de texto primero -- que es el que de
/// verdad importa para [EstabilizadorComprobanteRuta] -- y recién en su
/// `onComplete` se dispara el de barcode, cerrando el `ImageProxy` sólo
/// cuando ambos terminan.
private fun analizarComprobanteConBarcodeDebug(
    imagen: androidx.camera.core.ImageProxy,
    recognizer: com.google.mlkit.vision.text.TextRecognizer,
    barcodeScanner: com.google.mlkit.vision.barcode.BarcodeScanner,
    ejecutorPrincipal: java.util.concurrent.Executor,
    sesionActiva: AtomicBoolean,
    onTexto: (String) -> Unit,
    onBarcodes: (List<Barcode>) -> Unit,
    onFallo: () -> Unit,
) {
    val mediaImage = imagen.image
    if (mediaImage == null) {
        imagen.close()
        return
    }
    val rotacion = imagen.imageInfo.rotationDegrees
    val input = InputImage.fromMediaImage(mediaImage, rotacion)
    recognizer.process(input)
        .addOnSuccessListener(ejecutorPrincipal) { resultado ->
            if (sesionActiva.get()) onTexto(resultado.text)
        }
        .addOnFailureListener(ejecutorPrincipal) { if (sesionActiva.get()) onFallo() }
        .addOnCompleteListener(ejecutorPrincipal) {
            barcodeScanner.process(input)
                .addOnSuccessListener(ejecutorPrincipal) { codigos ->
                    if (sesionActiva.get()) onBarcodes(codigos.filter { it.rawValue != null })
                }
                .addOnCompleteListener(ejecutorPrincipal) {
                    imagen.close()
                }
        }
}

private const val MENSAJE_INICIAL = "Apunte al comprobante de carga de ruta"

