package com.brisas.controlacceso

import androidx.camera.core.Camera
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.FlashlightOff
import androidx.compose.material.icons.outlined.FlashlightOn
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.draw.clip
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

/// Lee el QR que muestra el panel al crear o re-vincular un dispositivo
/// (`brisas-acceso://vincular?codigo=…`, ver [CodigoVinculacion.desdeQr]):
/// así el código pasa de la pantalla del administrador al teléfono sin que
/// nadie lo copie ni lo mande por chat. Mucho más simple que las pantallas
/// de OCR: un QR se lee de una, sin encuadre ni votación, y el primer
/// código válido cierra el escáner.
@Composable
fun PantallaEscanearCodigoVinculacion(
    onCodigoLeido: (String) -> Unit,
    onEscribirCodigo: () -> Unit,
    onCerrar: () -> Unit,
) {
    RegistrarPantalla("escanear_codigo_vinculacion")
    EscanerConPermisoCamara(
        mensajePermiso = "Se necesita permiso de cámara para leer el código QR del panel.",
        onCerrar = onCerrar,
    ) {
        VistaCamaraQr(onCodigoLeido = onCodigoLeido, onEscribirCodigo = onEscribirCodigo, onCerrar = onCerrar)
    }
}

/// Lado del recuadro del visor, centrado en la vista de cámara. Es sólo una
/// guía: ML Kit lee el QR en cualquier parte del cuadro.
private val LadoVisor = 240.dp

@Composable
private fun VistaCamaraQr(onCodigoLeido: (String) -> Unit, onEscribirCodigo: () -> Unit, onCerrar: () -> Unit) {
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
    codigoLeido?.let { codigo ->
        LaunchedEffect(codigo) {
            delay(450)
            onCodigoLeido(codigo)
        }
    }
    val lector = remember {
        BarcodeScanning.getClient(BarcodeScannerOptions.Builder().setBarcodeFormats(Barcode.FORMAT_QR_CODE).build())
    }
    val hiloAnalisis = remember { Executors.newSingleThreadExecutor() }
    // Un solo aviso aunque lleguen varios frames con el mismo QR.
    val leido = remember { AtomicBoolean(false) }
    val previewView = remember { PreviewView(contexto) }
    var camara by remember { mutableStateOf<Camera?>(null) }
    var linterna by remember { mutableStateOf(false) }

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
                    camara = listo.bindToLifecycle(lifecycleOwner, CameraSelector.DEFAULT_BACK_CAMERA, preview, analisis)
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
        BotonCerrarCamara(onClick = onCerrar, modifier = Modifier.align(Alignment.TopEnd).padding(16.dp))
        // Sólo si la cámara tiene flash: con poca luz el QR de una pantalla
        // igual se lee, pero uno impreso puede necesitarla.
        if (camara?.cameraInfo?.hasFlashUnit() == true) {
            BotonLinterna(
                encendida = linterna,
                onClick = {
                    linterna = !linterna
                    camara?.cameraControl?.enableTorch(linterna)
                },
                modifier = Modifier.align(Alignment.TopStart).padding(16.dp),
            )
        }
        Column(
            modifier = Modifier.align(Alignment.BottomCenter).padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                "Apunte al código QR que muestra el panel",
                color = Color.White,
                style = MaterialTheme.typography.titleMedium,
            )
            TextButton(
                onClick = onEscribirCodigo,
                colors = ButtonDefaults.textButtonColors(contentColor = Color.White),
                modifier = Modifier.padding(top = 8.dp),
            ) {
                Text("Escribir el código")
            }
        }
    }
}

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

@Composable
private fun BotonLinterna(encendida: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    Box(
        modifier = modifier
            .size(44.dp)
            .clip(CircleShape)
            .background(if (encendida) Color.White else Color.Black.copy(alpha = 0.55f))
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            if (encendida) Icons.Outlined.FlashlightOff else Icons.Outlined.FlashlightOn,
            contentDescription = if (encendida) "Apagar linterna" else "Encender linterna",
            tint = if (encendida) Color.Black else Color.White,
        )
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
