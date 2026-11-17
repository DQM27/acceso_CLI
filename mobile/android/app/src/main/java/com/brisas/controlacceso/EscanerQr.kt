package com.brisas.controlacceso

import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.google.mlkit.vision.barcode.BarcodeScannerOptions
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.common.InputImage
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.delay

/// Cámara que lee un código QR: la comparten el código de vinculación del
/// panel y el QR del gafete PRAIND. Un QR se lee de una, sin encuadre ni
/// votación: el primer código que [interpretar] acepta (devuelve no nulo)
/// vibra, pinta el visor de verde un instante y llega a [onLeido]. Los QR
/// que [interpretar] rechaza se ignoran y la cámara sigue mirando.
/// [accionSecundaria] va debajo de la guía (ej. "Escribir el código").
/// El permiso de cámara lo pide quien la abre ([EscanerConPermisoCamara]).
@Composable
internal fun EscanerQr(
    textoGuia: String,
    interpretar: (String) -> String?,
    onLeido: (String) -> Unit,
    onCerrar: () -> Unit,
    accionSecundaria: (@Composable () -> Unit)? = null,
) {
    val contexto = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val haptica = LocalHapticFeedback.current
    // Al leer: vibración de confirmación y un instante con el check en el
    // visor antes de cerrar, para que se note que agarró el código.
    var codigoLeido by remember { mutableStateOf<String?>(null) }
    val onCodigoActual by rememberUpdatedState<(String) -> Unit>({ codigo ->
        haptica.performHapticFeedback(HapticFeedbackType.Confirm)
        codigoLeido = codigo
    })
    val interpretarActual by rememberUpdatedState(interpretar)
    codigoLeido?.let { codigo ->
        LaunchedEffect(codigo) {
            delay(450)
            onLeido(codigo)
        }
    }
    val lector = remember {
        BarcodeScanning.getClient(BarcodeScannerOptions.Builder().setBarcodeFormats(Barcode.FORMAT_QR_CODE).build())
    }
    val hiloAnalisis = remember { Executors.newSingleThreadExecutor() }
    // Un solo aviso aunque lleguen varios frames con el mismo QR.
    val leido = remember { AtomicBoolean(false) }
    val previewView = remember { PreviewView(contexto) }

    DisposableEffect(lifecycleOwner) {
        val proveedorFuturo = ProcessCameraProvider.getInstance(contexto)
        var proveedor: ProcessCameraProvider? = null
        proveedorFuturo.addListener(
            {
                val listo = runCatching { proveedorFuturo.get() }.getOrNull() ?: return@addListener
                proveedor = listo
                val preview = Preview.Builder().build().also { it.surfaceProvider = previewView.surfaceProvider }
                val analisis = ImageAnalysis.Builder()
                    .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                    .build()
                    .also { uso ->
                        uso.setAnalyzer(hiloAnalisis) { imagen ->
                            analizar(imagen, lector, leido, { interpretarActual(it) }) { codigo ->
                                ContextCompat.getMainExecutor(contexto).execute { onCodigoActual(codigo) }
                            }
                        }
                    }
                runCatching {
                    listo.unbindAll()
                    listo.bindToLifecycle(lifecycleOwner, CameraSelector.DEFAULT_BACK_CAMERA, preview, analisis)
                }
            },
            ContextCompat.getMainExecutor(contexto),
        )
        onDispose {
            proveedor?.unbindAll()
            lector.close()
            hiloAnalisis.shutdown()
        }
    }

    Box(modifier = Modifier.fillMaxSize()) {
        AndroidView(factory = { previewView }, modifier = Modifier.fillMaxSize())
        VisorQr(leido = codigoLeido != null, modifier = Modifier.fillMaxSize())
        Column(
            modifier = Modifier.align(Alignment.BottomCenter).padding(top = 24.dp, start = 24.dp, end = 24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                textoGuia,
                color = Color.White,
                style = MaterialTheme.typography.titleMedium,
                textAlign = TextAlign.Center,
            )
            accionSecundaria?.invoke()
            // Cerrar abajo al centro, como en las demás cámaras (antes, arriba
            // a la derecha, la esquina más lejos del pulgar).
            BotonCerrarCamara(onClick = onCerrar, modifier = Modifier.padding(top = 12.dp, bottom = 28.dp))
        }
    }
}

/// Lado del recuadro del visor, centrado en la vista de cámara. Es sólo una
/// guía: ML Kit lee el QR en cualquier parte del cuadro.
private val LadoVisor = 240.dp

/// Oscurece todo menos un recuadro redondeado al centro, con las esquinas
/// marcadas; al leer, el marco pasa a verde.
@Composable
private fun VisorQr(leido: Boolean, modifier: Modifier = Modifier) {
    val colorMarco by animateColorAsState(if (leido) ColorExitoBrisas else Color.White, label = "marcoVisor")
    Canvas(modifier = modifier.graphicsLayer { compositingStrategy = CompositingStrategy.Offscreen }) {
        val lado = LadoVisor.toPx()
        val esquina = Offset((size.width - lado) / 2f, (size.height - lado) / 2f)
        val radio = CornerRadius(24.dp.toPx())
        drawRect(Color.Black.copy(alpha = 0.55f))
        drawRoundRect(Color.Transparent, esquina, Size(lado, lado), radio, blendMode = BlendMode.Clear)
        drawRoundRect(
            colorMarco,
            esquina,
            Size(lado, lado),
            radio,
            style = Stroke(width = 4.dp.toPx()),
        )
    }
}

@androidx.annotation.OptIn(androidx.camera.core.ExperimentalGetImage::class)
private fun analizar(
    imagen: ImageProxy,
    lector: com.google.mlkit.vision.barcode.BarcodeScanner,
    leido: AtomicBoolean,
    interpretar: (String) -> String?,
    onCodigo: (String) -> Unit,
) {
    val medio = imagen.image
    if (medio == null || leido.get()) {
        imagen.close()
        return
    }
    lector.process(InputImage.fromMediaImage(medio, imagen.imageInfo.rotationDegrees))
        .addOnSuccessListener { codigos ->
            val codigo = codigos.firstNotNullOfOrNull { it.rawValue?.let(interpretar) }
            if (codigo != null && leido.compareAndSet(false, true)) onCodigo(codigo)
        }
        .addOnCompleteListener { imagen.close() }
}
