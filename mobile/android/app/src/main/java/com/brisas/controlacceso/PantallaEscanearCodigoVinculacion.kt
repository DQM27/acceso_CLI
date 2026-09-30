package com.brisas.controlacceso

import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
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

/// Lee el QR que muestra el panel al crear o re-vincular un dispositivo
/// (`brisas-acceso://vincular?codigo=…`, ver [CodigoVinculacion.desdeQr]):
/// así el código pasa de la pantalla del administrador al teléfono sin que
/// nadie lo copie ni lo mande por chat. Mucho más simple que las pantallas
/// de OCR: un QR se lee de una, sin encuadre ni votación, y el primer
/// código válido cierra el escáner.
@Composable
fun PantallaEscanearCodigoVinculacion(
    onCodigoLeido: (String) -> Unit,
    onCerrar: () -> Unit,
) {
    RegistrarPantalla("escanear_codigo_vinculacion")
    EscanerConPermisoCamara(
        mensajePermiso = "Se necesita permiso de cámara para leer el código QR del panel.",
        onCerrar = onCerrar,
    ) {
        VistaCamaraQr(onCodigoLeido = onCodigoLeido, onCerrar = onCerrar)
    }
}

@Composable
private fun VistaCamaraQr(onCodigoLeido: (String) -> Unit, onCerrar: () -> Unit) {
    val contexto = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val onCodigoActual by rememberUpdatedState(onCodigoLeido)
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
                            analizar(imagen, lector, leido) { codigo ->
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
        Text(
            "Apuntá al código QR que muestra el panel",
            color = Color.White,
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.align(Alignment.BottomCenter).padding(32.dp),
        )
        BotonCerrarCamara(onClick = onCerrar, modifier = Modifier.align(Alignment.TopEnd).padding(16.dp))
    }
}

@androidx.annotation.OptIn(androidx.camera.core.ExperimentalGetImage::class)
private fun analizar(
    imagen: ImageProxy,
    lector: com.google.mlkit.vision.barcode.BarcodeScanner,
    leido: AtomicBoolean,
    onCodigo: (String) -> Unit,
) {
    val medio = imagen.image
    if (medio == null || leido.get()) {
        imagen.close()
        return
    }
    lector.process(InputImage.fromMediaImage(medio, imagen.imageInfo.rotationDegrees))
        .addOnSuccessListener { codigos ->
            val codigo = codigos.firstNotNullOfOrNull { it.rawValue?.let(CodigoVinculacion::desdeQr) }
            if (codigo != null && leido.compareAndSet(false, true)) onCodigo(codigo)
        }
        .addOnCompleteListener { imagen.close() }
}
