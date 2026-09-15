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
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

/// Escaneo del paso 3 del checklist ("Placa o número de unidad", ver
/// `PantallaRutas.kt` y `LectorVehiculoRuta.kt`) -- mismo esqueleto que
/// [PantallaEscanearComprobanteRuta]. Un solo escaneo cubre los dos casos
/// (calcomanía de número de unidad de un camión de flota, o placa de un
/// camión de apoyo): [extraerVehiculo] decide cuál es cuál, esta pantalla
/// no necesita saberlo de antemano.
@Composable
fun PantallaEscanearVehiculoRuta(
    onVehiculoDetectado: suspend (VehiculoRutaDetectado) -> Unit,
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
        VistaCamaraVehiculoRuta(onVehiculoDetectado = onVehiculoDetectado, onCerrar = onCerrar)
    } else {
        Column(
            modifier = Modifier.fillMaxSize().padding(16.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                "Se necesita permiso de cámara para escanear la placa o el número de unidad.",
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
private fun VistaCamaraVehiculoRuta(
    onVehiculoDetectado: suspend (VehiculoRutaDetectado) -> Unit,
    onCerrar: () -> Unit,
) {
    val contexto = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val alcance = rememberCoroutineScope()
    val onDetectadoActual by rememberUpdatedState(onVehiculoDetectado)
    val haptica = LocalHapticFeedback.current
    val ejecutor = remember { Executors.newSingleThreadExecutor() }
    val recognizer = remember { TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS) }
    var ultimoMensaje by remember { mutableStateOf(MENSAJE_INICIAL_VEHICULO) }
    var estado by remember { mutableStateOf(EstadoEscaneo.BUSCANDO) }
    var textoCrudoDebug by remember { mutableStateOf("") }
    val estabilizador = remember { EstabilizadorVehiculoRuta() }
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
                            analizarCedula(
                                imagen = imagen,
                                recognizer = recognizer,
                                ejecutorPrincipal = ejecutorPrincipal,
                                sesionActiva = sesionActiva,
                                onTexto = { texto ->
                                    if (sesionActiva.get()) {
                                        if (BuildConfig.DEBUG) textoCrudoDebug = texto
                                        val resultado = estabilizador.procesarFrame(texto)
                                        if (resultado != null) {
                                            estado = EstadoEscaneo.CONFIRMADO
                                            ultimoMensaje = "${resultado.valor} confirmado"
                                            if (detectada.compareAndSet(false, true)) {
                                                haptica.performHapticFeedback(HapticFeedbackType.Confirm)
                                                trabajoResultado?.cancel()
                                                trabajoResultado = alcance.launch {
                                                    if (sesionActiva.get()) onDetectadoActual(resultado)
                                                }
                                            }
                                        } else {
                                            estado = EstadoEscaneo.BUSCANDO
                                            ultimoMensaje = MENSAJE_INICIAL_VEHICULO
                                        }
                                    }
                                },
                                onFallo = {
                                    if (sesionActiva.get()) ultimoMensaje = "No se pudo leer el texto. Intente acercar."
                                },
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
        if (BuildConfig.DEBUG && textoCrudoDebug.isNotBlank()) {
            Text(
                "DEBUG -- texto crudo de ML Kit:\n$textoCrudoDebug",
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

private const val MENSAJE_INICIAL_VEHICULO = "Apunte a la placa o al número de unidad"

/// Debounce por repetición de frames -- mismo criterio que
/// `EstabilizadorComprobanteRuta`/`EstabilizadorCarnetKof`.
private class EstabilizadorVehiculoRuta(
    private val framesRequeridos: Int = 2,
    private val ventana: Int = framesRequeridos + 2,
) {
    private val candidatosRecientes = ArrayDeque<String>()

    fun procesarFrame(texto: String): VehiculoRutaDetectado? {
        val detectado = extraerVehiculo(texto)
        val clave = detectado?.let { "${it.tipo}:${it.valor}" } ?: CLAVE_SIN_CANDIDATO
        candidatosRecientes.addLast(clave)
        while (candidatosRecientes.size > ventana) candidatosRecientes.removeFirst()
        if (detectado == null) return null
        val repeticiones = candidatosRecientes.count { it == clave }
        return if (repeticiones >= framesRequeridos) detectado else null
    }

    companion object {
        private const val CLAVE_SIN_CANDIDATO = " "
    }
}
