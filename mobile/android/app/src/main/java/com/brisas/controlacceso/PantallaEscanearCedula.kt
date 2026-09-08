package com.brisas.controlacceso

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.ExperimentalGetImage
import androidx.camera.core.FocusMeteringAction
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.core.SurfaceOrientedMeteringPointFactory
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
    var ultimoMensaje by remember { mutableStateOf("Alinee el documento dentro de la cámara") }
    var estado by remember { mutableStateOf(EstadoEscaneo.BUSCANDO) }
    // Una instancia por apertura de pantalla -- lleva el conteo de frames
    // consistentes del debounce (ver EstabilizadorLectura), no debe
    // compartirse entre sesiones de escaneo distintas.
    val estabilizador = remember { EstabilizadorLectura() }
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

    // Colores de estado fijos, no dependientes del tema (Classic/Brisas/Negro):
    // acá el color comunica significado (buscando/inválido/confirmado), y ese
    // significado debe leerse igual sin importar qué tema tenga activo quien
    // opera -- a diferencia del acento decorativo que usaba antes este marco.
    val colorBuscando = Color(0xFF9E9E9E)
    val colorInvalido = Color(0xFFE53935)
    val colorConfirmado = Color(0xFF43A047)
    val colorMarco = when (estado) {
        EstadoEscaneo.BUSCANDO -> colorBuscando
        EstadoEscaneo.INVALIDO -> colorInvalido
        EstadoEscaneo.CONFIRMADO -> colorConfirmado
    }

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
                                        onTexto = { texto ->
                                            val resultado = estabilizador.procesarFrame(texto)
                                            estado = resultado.estado
                                            ultimoMensaje = resultado.mensaje
                                            val documento = resultado.documento
                                            if (resultado.estado == EstadoEscaneo.CONFIRMADO && documento != null) {
                                                if (detectada.compareAndSet(false, true)) {
                                                    onCedulaDetectada(documento.numeroDocumento)
                                                }
                                            }
                                        },
                                        onFallo = {
                                            estado = EstadoEscaneo.BUSCANDO
                                            ultimoMensaje = "No se pudo leer el texto. Intente acercar el documento."
                                        },
                                    )
                                }
                            }
                        proveedor.unbindAll()
                        val camara = proveedor.bindToLifecycle(
                            lifecycleOwner,
                            CameraSelector.DEFAULT_BACK_CAMERA,
                            preview,
                            analisis,
                        )

                        // Enfoque continuo en el centro (donde vive el
                        // recuadro guía) en vez de confiar en el autofocus
                        // por defecto: a la distancia típica de escaneo
                        // (10-15cm) muchos dispositivos no enfocan bien sin
                        // esta ayuda -- ver plan, sección 0.6. Se usa un
                        // factory normalizado (0..1) en vez de las
                        // dimensiones reales de `previewView` porque en este
                        // punto su layout todavía puede no tener tamaño.
                        val puntoCentral = SurfaceOrientedMeteringPointFactory(1f, 1f)
                            .createPoint(0.5f, 0.5f)
                        val accionEnfoque = FocusMeteringAction.Builder(puntoCentral, FocusMeteringAction.FLAG_AF)
                            .disableAutoCancel()
                            .build()
                        camara.cameraControl.startFocusAndMetering(accionEnfoque)
                    },
                    ContextCompat.getMainExecutor(ctx),
                )
                previewView
            },
            modifier = Modifier.fillMaxSize(),
        )
        MarcoGuiaCedula(color = colorMarco, modifier = Modifier.fillMaxSize())
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

/// Recorte oscuro + marco de esquinas al estilo "encuadre de escáner". Ya no
/// es sólo decorativo: [analizarCedula] filtra el texto de ML Kit contra
/// esta misma área (ver [filtrarTextoEnAreaGuia]) -- si las proporciones de
/// acá cambian, deben cambiar junto con las de esa función. Proporción
/// 1.586:1, la misma de una tarjeta ID-1 (cédula/carnet), no un cuadrado
/// genérico.
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

/// Sólo entrega a ML Kit y devuelve el texto YA recortado al área guía --
/// la clasificación de tipo de documento, extracción de campos y decisión
/// de aceptar o no la lectura viven en [EstabilizadorLectura], no acá
/// (separar esto evita que esta función termine "sabiendo" de
/// cédulas/DIMEX/licencias/MRZ).
///
/// El recorte (plan, sección 9) se hace filtrando los `TextBlock` de ML Kit
/// por su `boundingBox` contra el recuadro guía -- no convirtiendo el frame
/// a Bitmap para recortar píxeles, ver [filtrarTextoEnAreaGuia].
@androidx.annotation.OptIn(ExperimentalGetImage::class)
private fun analizarCedula(
    imagen: ImageProxy,
    recognizer: com.google.mlkit.vision.text.TextRecognizer,
    onTexto: (String) -> Unit,
    onFallo: () -> Unit,
) {
    val mediaImage = imagen.image
    if (mediaImage == null) {
        imagen.close()
        return
    }
    val rotacion = imagen.imageInfo.rotationDegrees
    val (anchoUpright, altoUpright) = dimensionesUpright(mediaImage.width, mediaImage.height, rotacion)
    val input = InputImage.fromMediaImage(mediaImage, rotacion)
    recognizer.process(input)
        .addOnSuccessListener { resultado ->
            val bloques = resultado.textBlocks.map { bloque ->
                val caja = bloque.boundingBox?.let { CajaTexto(it.left, it.top, it.right, it.bottom) }
                BloqueTextoOcr(caja, bloque.text)
            }
            onTexto(filtrarTextoEnAreaGuia(bloques, anchoUpright, altoUpright))
        }
        .addOnFailureListener { onFallo() }
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
