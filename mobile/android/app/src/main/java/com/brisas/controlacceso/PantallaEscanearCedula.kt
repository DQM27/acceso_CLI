package com.brisas.controlacceso

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.ExperimentalGetImage
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

@Composable
fun PantallaEscanearCedula(onCedulaDetectada: (String) -> Unit, onCerrar: () -> Unit) {
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
        VistaCamaraCedula(onCedulaDetectada = onCedulaDetectada, onCerrar = onCerrar)
    } else {
        Column(
            modifier = Modifier.fillMaxSize().padding(16.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text("Se necesita permiso de cámara para escanear cédulas.", color = MaterialTheme.colorScheme.onSurfaceVariant)
            Row(modifier = Modifier.padding(top = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                BotonDiscretoBrisas(onClick = onCerrar) {
                    Text("Volver")
                }
                BotonBrisas(onClick = { pedirPermiso.launch(Manifest.permission.CAMERA) }) {
                    Text("Dar permiso")
                }
            }
        }
    }
}

@Composable
private fun VistaCamaraCedula(onCedulaDetectada: (String) -> Unit, onCerrar: () -> Unit) {
    val contexto = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val ejecutor = remember { Executors.newSingleThreadExecutor() }
    val recognizer = remember { TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS) }
    var ultimoMensaje by remember { mutableStateOf("Alinee la cédula dentro de la cámara") }
    // AtomicBoolean, no `mutableStateOf` -- esta bandera se lee en el hilo
    // del analizador de cámara (`ejecutor`) y se escribe desde el hilo
    // principal (callback de ML Kit); un booleano de Compose no garantiza
    // esa visibilidad entre hilos, y además el `compareAndSet` evita que
    // dos frames en vuelo disparen `onCedulaDetectada` dos veces.
    val detectada = remember { AtomicBoolean(false) }
    // Guardado acá para poder desatarlo explícitamente al salir -- `bindToLifecycle`
    // por sí solo no alcanza: en una app de una sola Activity con Compose,
    // `LocalLifecycleOwner` suele ser la Activity, no esta pantalla, así que la
    // cámara no se libera sola al navegar fuera de acá, sólo al morir la Activity.
    // Sin este `unbindAll()` explícito, reabrir el escáner puede encontrar la
    // cámara todavía atada al ciclo de vida anterior.
    var cameraProvider by remember { mutableStateOf<ProcessCameraProvider?>(null) }

    DisposableEffect(Unit) {
        onDispose {
            cameraProvider?.unbindAll()
            ejecutor.shutdown()
            recognizer.close()
        }
    }

    // Capturado acá (fuera de Canvas, que no es @Composable) para que el
    // marco respete el color de acento del tema activo, en vez de un color
    // fijo que desentone con Classic/Brisas/Negro.
    val colorAcento = MaterialTheme.colorScheme.primary

    Box(modifier = Modifier.fillMaxSize()) {
        AndroidView(
            factory = { ctx ->
                val previewView = PreviewView(ctx).apply {
                    scaleType = PreviewView.ScaleType.FILL_CENTER
                }
                val cameraProviderFuture = ProcessCameraProvider.getInstance(ctx)
                cameraProviderFuture.addListener(
                    {
                        val proveedor = cameraProviderFuture.get()
                        cameraProvider = proveedor
                        val preview = Preview.Builder().build().also {
                            it.surfaceProvider = previewView.surfaceProvider
                        }
                        val analisis = ImageAnalysis.Builder()
                            .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                            .build()
                            .also {
                                it.setAnalyzer(ejecutor) { imagen ->
                                    // Ya se detectó una cédula y se avisó al
                                    // llamador -- seguir corriendo ML Kit en
                                    // cada frame mientras la pantalla termina
                                    // de cerrarse sólo quema CPU sin ganar
                                    // nada (el resultado ya se usó).
                                    if (detectada.get()) {
                                        imagen.close()
                                        return@setAnalyzer
                                    }
                                    analizarCedula(
                                        imagen = imagen,
                                        recognizer = recognizer,
                                        onTexto = { ultimoMensaje = it },
                                        onCedula = { cedula ->
                                            if (detectada.compareAndSet(false, true)) {
                                                onCedulaDetectada(cedula)
                                            }
                                        },
                                    )
                                }
                            }
                        proveedor.unbindAll()
                        proveedor.bindToLifecycle(
                            lifecycleOwner,
                            CameraSelector.DEFAULT_BACK_CAMERA,
                            preview,
                            analisis,
                        )
                    },
                    ContextCompat.getMainExecutor(ctx),
                )
                previewView
            },
            modifier = Modifier.fillMaxSize(),
        )
        MarcoGuiaCedula(color = colorAcento, modifier = Modifier.fillMaxSize())
        Column(
            modifier = Modifier.fillMaxWidth().align(Alignment.TopCenter).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(
                ultimoMensaje,
                color = MaterialTheme.colorScheme.onPrimary,
                style = MaterialTheme.typography.bodyLarge,
                modifier = Modifier.fillMaxWidth(),
            )
            BotonDiscretoBrisas(onClick = onCerrar) {
                Text("Cancelar")
            }
        }
    }
}

/// Recorte oscuro + marco de esquinas al estilo "encuadre de escáner",
/// puramente decorativo -- no restringe ni recorta lo que ML Kit analiza
/// (sigue leyendo el frame completo, ver [analizarCedula]), sólo le indica
/// visualmente a quien opera dónde alinear la cédula. Proporción 1.586:1,
/// la misma de una tarjeta ID-1 (cédula/carnet), no un cuadrado genérico.
@Composable
private fun MarcoGuiaCedula(color: Color, modifier: Modifier = Modifier) {
    Canvas(modifier = modifier) {
        val anchoMarco = size.width * 0.82f
        val altoMarco = anchoMarco / 1.586f
        val izquierda = (size.width - anchoMarco) / 2f
        val arriba = (size.height - altoMarco) / 2f
        val radio = CornerRadius(20.dp.toPx())

        // `saveLayer` + `BlendMode.Clear` recorta un hueco transparente en
        // el velo oscuro -- dibujar el rectángulo directo con alpha no
        // sirve, dejaría ver el velo encima de la vista previa también
        // dentro del marco.
        drawIntoCanvas { canvas ->
            val capa = canvas.nativeCanvas.saveLayer(null, null)
            drawRect(color = Color.Black.copy(alpha = 0.55f))
            drawRoundRect(
                color = Color.Transparent,
                topLeft = Offset(izquierda, arriba),
                size = Size(anchoMarco, altoMarco),
                cornerRadius = radio,
                blendMode = BlendMode.Clear,
            )
            canvas.nativeCanvas.restoreToCount(capa)
        }

        drawRoundRect(
            color = Color.White.copy(alpha = 0.85f),
            topLeft = Offset(izquierda, arriba),
            size = Size(anchoMarco, altoMarco),
            cornerRadius = radio,
            style = Stroke(width = 1.5.dp.toPx()),
        )

        // Cuatro esquinas acentuadas, más gruesas que el borde fino de
        // arriba -- lo que el ojo realmente sigue al alinear la cédula.
        val largoEsquina = 26.dp.toPx()
        val grosor = 4.dp.toPx()
        val derecha = izquierda + anchoMarco
        val abajo = arriba + altoMarco
        val esquinas = listOf(
            // (vértice, hacia la derecha en X, hacia abajo en Y)
            Triple(Offset(izquierda, arriba), true, true),
            Triple(Offset(derecha, arriba), false, true),
            Triple(Offset(izquierda, abajo), true, false),
            Triple(Offset(derecha, abajo), false, false),
        )
        for ((vertice, haciaDerecha, haciaAbajo) in esquinas) {
            val dx = if (haciaDerecha) largoEsquina else -largoEsquina
            val dy = if (haciaAbajo) largoEsquina else -largoEsquina
            drawLine(
                color = color,
                start = vertice,
                end = Offset(vertice.x + dx, vertice.y),
                strokeWidth = grosor,
                cap = StrokeCap.Round,
            )
            drawLine(
                color = color,
                start = vertice,
                end = Offset(vertice.x, vertice.y + dy),
                strokeWidth = grosor,
                cap = StrokeCap.Round,
            )
        }
    }
}

@androidx.annotation.OptIn(ExperimentalGetImage::class)
private fun analizarCedula(
    imagen: ImageProxy,
    recognizer: com.google.mlkit.vision.text.TextRecognizer,
    onTexto: (String) -> Unit,
    onCedula: (String) -> Unit,
) {
    val mediaImage = imagen.image
    if (mediaImage == null) {
        imagen.close()
        return
    }
    val input = InputImage.fromMediaImage(mediaImage, imagen.imageInfo.rotationDegrees)
    recognizer.process(input)
        .addOnSuccessListener { resultado ->
            val cedula = extraerCedulaDeTexto(resultado.text)
            if (cedula != null) {
                onTexto("Cédula detectada: $cedula")
                onCedula(cedula)
            } else {
                onTexto("Buscando número de cédula…")
            }
        }
        .addOnFailureListener {
            onTexto("No se pudo leer el texto. Intente acercar la cédula.")
        }
        .addOnCompleteListener {
            imagen.close()
        }
}

fun extraerCedulaDeTexto(texto: String): String? {
    val normalizado = texto.replace(Regex("[^0-9\\n -]"), " ")
    Regex("""\b\d[- ]?\d{4}[- ]?\d{4}\b""")
        .find(normalizado)
        ?.let { return it.value.filter(Char::isDigit) }

    return Regex("""\b\d{9}\b""")
        .find(normalizado)
        ?.value
}
