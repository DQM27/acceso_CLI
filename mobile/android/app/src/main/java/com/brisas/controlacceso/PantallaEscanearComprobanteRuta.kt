package com.brisas.controlacceso

import android.Manifest
import android.content.pm.PackageManager
import android.util.Size
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
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
    BackHandler(onBack = onCerrar)
    val contexto = LocalContext.current
    var permisoConcedido by remember {
        mutableStateOf(ContextCompat.checkSelfPermission(contexto, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED)
    }
    val pedirPermiso = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { concedido ->
        permisoConcedido = concedido
    }

    LaunchedEffect(Unit) {
        if (!permisoConcedido) {
            pedirPermiso.launch(Manifest.permission.CAMERA)
        }
    }

    if (permisoConcedido) {
        VistaCamaraComprobanteRuta(onComprobanteDetectado = onComprobanteDetectado, onCerrar = onCerrar)
    } else {
        Column(
            modifier = Modifier.fillMaxSize().padding(16.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                "Se necesita permiso de cámara para escanear el comprobante.",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Row(modifier = Modifier.padding(top = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                BotonDiscretoBrisas(onClick = onCerrar) { Text("Volver") }
                BotonBrisas(onClick = { pedirPermiso.launch(Manifest.permission.CAMERA) }) { Text("Dar permiso") }
            }
        }
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
    // Sólo en debug (mismo criterio que FLAG_SECURE en MainActivity.kt) --
    // hasta tener el perfil probado a fondo contra el papel real, ver el
    // texto crudo de ML Kit en pantalla es la forma más rápida de ajustar
    // los regex de LectorComprobanteRuta.kt sin adivinar a ciegas.
    var textoCrudoDebug by remember { mutableStateOf("") }
    var barcodeCrudoDebug by remember { mutableStateOf("") }
    val estabilizador = remember { EstabilizadorComprobanteRuta() }
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

    val colorBuscando = Color(0xFF9E9E9E)
    val colorConfirmado = Color(0xFF43A047)
    val colorMarco = if (estado == EstadoEscaneo.CONFIRMADO) colorConfirmado else colorBuscando

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
                                    if (BuildConfig.DEBUG) textoCrudoDebug = texto
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
                                if (sesionActiva.get()) ultimoMensaje = "No se pudo leer el texto. Intente acercar."
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
                                    onBarcodes = { valores ->
                                        if (sesionActiva.get() && valores.isNotEmpty()) {
                                            barcodeCrudoDebug = valores.joinToString("\n")
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
        Column(
            modifier = Modifier.fillMaxWidth().align(Alignment.TopCenter).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(
                ultimoMensaje,
                color = Color.White,
                style = MaterialTheme.typography.bodyLarge,
                modifier = Modifier
                    .fillMaxWidth()
                    .background(Color.Black.copy(alpha = 0.78f))
                    .padding(12.dp),
            )
            BotonDiscretoBrisas(onClick = onCerrar) { Text("Cancelar") }
        }
        if (BuildConfig.DEBUG && (textoCrudoDebug.isNotBlank() || barcodeCrudoDebug.isNotBlank())) {
            val textoDebug = buildString {
                if (barcodeCrudoDebug.isNotBlank()) append("DEBUG -- código de barras:\n$barcodeCrudoDebug\n\n")
                if (textoCrudoDebug.isNotBlank()) append("DEBUG -- texto crudo de ML Kit:\n$textoCrudoDebug")
            }
            Text(
                textoDebug,
                color = Color.White,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier
                    .fillMaxWidth()
                    .align(Alignment.BottomCenter)
                    .heightIn(max = 320.dp)
                    .verticalScroll(rememberScrollState())
                    .background(Color.Black.copy(alpha = 0.85f))
                    .padding(12.dp),
            )
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
    onBarcodes: (List<String>) -> Unit,
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
                    if (sesionActiva.get()) {
                        val valores = codigos.mapNotNull { codigo: Barcode ->
                            codigo.rawValue?.let { valor -> "$valor (formato ${codigo.format})" }
                        }
                        onBarcodes(valores)
                    }
                }
                .addOnCompleteListener(ejecutorPrincipal) {
                    imagen.close()
                }
        }
}

private const val MENSAJE_INICIAL = "Apunte al comprobante de carga de ruta"

/// Debounce simple por repetición de frames -- mismo criterio que
/// `EstabilizadorLectura` (ver ese archivo), pero sin MRZ ni tipos de
/// documento de identidad: la clave estable acá es ruta+sub-número+
/// documento juntos, ya que los tres vienen impresos en la misma línea y
/// deben leerse consistentes entre sí, no por separado.
private class EstabilizadorComprobanteRuta(
    private val framesRequeridos: Int = 2,
    private val ventana: Int = framesRequeridos + 2,
) {
    private val candidatosRecientes = ArrayDeque<String>()

    fun procesarFrame(texto: String): ComprobanteRutaDetectado? {
        val detectado = extraerComprobanteRuta(texto)
        val clave = detectado?.let { "${it.numeroRuta}:${it.subNumero}:${it.numeroDocumento}" } ?: CLAVE_SIN_CANDIDATO
        candidatosRecientes.addLast(clave)
        while (candidatosRecientes.size > ventana) candidatosRecientes.removeFirst()
        if (detectado == null) return null
        val repeticiones = candidatosRecientes.count { it == clave }
        return if (repeticiones >= framesRequeridos) detectado else null
    }

    companion object {
        private const val CLAVE_SIN_CANDIDATO = " "
    }
}
