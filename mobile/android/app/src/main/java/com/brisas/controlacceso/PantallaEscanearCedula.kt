package com.brisas.controlacceso

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
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
import androidx.compose.ui.graphics.Color
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
    var vencido by remember { mutableStateOf(false) }
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
    // Ver nota en `analizarCedula`: se pasa explícito en vez de dejar que
    // ML Kit use su executor por defecto de forma implícita.
    val ejecutorPrincipal = remember { ContextCompat.getMainExecutor(contexto) }

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
    // Un documento vencido igual se leyó bien (por eso el estado sigue
    // siendo CONFIRMADO, no INVALIDO), pero visualmente no puede quedar
    // idéntico a uno vigente -- ámbar, ni el verde de "todo bien" ni el
    // rojo de "no reconocido".
    val colorVencido = Color(0xFFFF8F00)
    val colorMarco = when {
        estado == EstadoEscaneo.CONFIRMADO && vencido -> colorVencido
        estado == EstadoEscaneo.CONFIRMADO -> colorConfirmado
        estado == EstadoEscaneo.INVALIDO -> colorInvalido
        else -> colorBuscando
    }

    Box(modifier = Modifier.fillMaxSize()) {
        AndroidView(
            factory = { ctx ->
                val previewView = PreviewView(ctx).apply {
                    scaleType = PreviewView.ScaleType.FILL_CENTER
                }
                val analisis = construirAnalizadorOcr(
                    ejecutorAnalisis = ejecutor,
                    ejecutorPrincipal = ejecutorPrincipal,
                    recognizer = recognizer,
                    estabilizador = estabilizador,
                    detectada = detectada,
                    onResultado = { resultado ->
                        estado = resultado.estado
                        ultimoMensaje = resultado.mensaje
                        vencido = resultado.vencido
                        val documento = resultado.documento
                        if (resultado.estado == EstadoEscaneo.CONFIRMADO && documento != null) {
                            if (detectada.compareAndSet(false, true)) {
                                vibrarConfirmacion(contexto)
                                onCedulaDetectada(documento.numeroDocumento)
                            }
                        }
                    },
                    onFallo = {
                        estado = EstadoEscaneo.BUSCANDO
                        vencido = false
                        ultimoMensaje = "No se pudo leer el texto. Intente acercar el documento."
                    },
                )
                iniciarCamara(
                    ctx = ctx,
                    previewView = previewView,
                    lifecycleOwner = lifecycleOwner,
                    analisis = analisis,
                    onCameraProviderListo = { cameraProvider = it },
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

/// Arma el caso de uso de análisis de ML Kit, incluyendo el descarte
/// temprano de frames una vez ya se detectó un documento -- separado de
/// [iniciarCamara] para que cada función tenga una sola responsabilidad:
/// esta arma "qué se analiza", la otra "cómo se conecta a la cámara física".
private fun construirAnalizadorOcr(
    ejecutorAnalisis: java.util.concurrent.Executor,
    ejecutorPrincipal: java.util.concurrent.Executor,
    recognizer: com.google.mlkit.vision.text.TextRecognizer,
    estabilizador: EstabilizadorLectura,
    detectada: AtomicBoolean,
    onResultado: (ResultadoEstabilizacion) -> Unit,
    onFallo: () -> Unit,
): ImageAnalysis =
    ImageAnalysis.Builder()
        .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
        .build()
        .also { analisis ->
            analisis.setAnalyzer(ejecutorAnalisis) { imagen ->
                // Ya se detectó un documento y se avisó al llamador --
                // seguir corriendo ML Kit en cada frame mientras la
                // pantalla termina de cerrarse sólo quema CPU sin ganar
                // nada (el resultado ya se usó).
                if (detectada.get()) {
                    imagen.close()
                    return@setAnalyzer
                }
                analizarCedula(
                    imagen = imagen,
                    recognizer = recognizer,
                    ejecutorPrincipal = ejecutorPrincipal,
                    onTexto = { texto -> onResultado(estabilizador.procesarFrame(texto)) },
                    onFallo = onFallo,
                )
            }
        }

/// Conecta el preview y el análisis a la cámara física una vez que
/// `ProcessCameraProvider` está listo, y arranca el enfoque continuo --
/// todo lo que depende de esa espera asíncrona vive acá, separado de cómo
/// se arma el analizador ([construirAnalizadorOcr]).
private fun iniciarCamara(
    ctx: android.content.Context,
    previewView: PreviewView,
    lifecycleOwner: androidx.lifecycle.LifecycleOwner,
    analisis: ImageAnalysis,
    onCameraProviderListo: (ProcessCameraProvider) -> Unit,
) {
    val cameraProviderFuture = ProcessCameraProvider.getInstance(ctx)
    cameraProviderFuture.addListener(
        {
            val proveedor = cameraProviderFuture.get()
            onCameraProviderListo(proveedor)
            val preview = Preview.Builder().build().also {
                it.surfaceProvider = previewView.surfaceProvider
            }
            proveedor.unbindAll()
            val camara = proveedor.bindToLifecycle(
                lifecycleOwner,
                CameraSelector.DEFAULT_BACK_CAMERA,
                preview,
                analisis,
            )
            enfocarCentro(camara)
        },
        ContextCompat.getMainExecutor(ctx),
    )
}

/// Enfoque continuo en el centro (donde vive el recuadro guía) en vez de
/// confiar en el autofocus por defecto: a la distancia típica de escaneo
/// (10-15cm) muchos dispositivos no enfocan bien sin esta ayuda -- ver
/// plan, sección 0.6. Se usa un factory normalizado (0..1) en vez de las
/// dimensiones reales de `previewView` porque en este punto su layout
/// todavía puede no tener tamaño.
private fun enfocarCentro(camara: androidx.camera.core.Camera) {
    val puntoCentral = SurfaceOrientedMeteringPointFactory(1f, 1f).createPoint(0.5f, 0.5f)
    val accionEnfoque = FocusMeteringAction.Builder(puntoCentral, FocusMeteringAction.FLAG_AF)
        .disableAutoCancel()
        .build()
    camara.cameraControl.startFocusAndMetering(accionEnfoque)
}

/// Vibración corta de confirmación (plan, sección 7-8: "check verde +
/// vibración corta" al aceptar una lectura) -- señal táctil de que ya
/// terminó, para no depender solo del color del marco o del mensaje en
/// pantalla. `minSdk` de la app es 26, así que `VibrationEffect` siempre
/// existe; sólo cambia de dónde se obtiene el `Vibrator` según la versión.
private fun vibrarConfirmacion(contexto: Context) {
    val vibrator = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        (contexto.getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as? VibratorManager)?.defaultVibrator
    } else {
        @Suppress("DEPRECATION")
        contexto.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
    }
    vibrator?.vibrate(VibrationEffect.createOneShot(80, VibrationEffect.DEFAULT_AMPLITUDE))
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
///
/// `ejecutorPrincipal` se pasa explícito a los listeners en vez de dejar que
/// la Tasks API use su default (que también es el hilo principal, pero de
/// forma implícita) -- `onTexto` termina llamando a `EstabilizadorLectura`,
/// que no es thread-safe y asume ejecución serializada en un único hilo; más
/// vale que esa garantía sea explícita acá que depender de un comportamiento
/// por defecto de una librería externa.
@androidx.annotation.OptIn(ExperimentalGetImage::class)
private fun analizarCedula(
    imagen: ImageProxy,
    recognizer: com.google.mlkit.vision.text.TextRecognizer,
    ejecutorPrincipal: java.util.concurrent.Executor,
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
        .addOnSuccessListener(ejecutorPrincipal) { resultado ->
            val bloques = resultado.textBlocks.map { bloque ->
                val caja = bloque.boundingBox?.let { CajaTexto(it.left, it.top, it.right, it.bottom) }
                BloqueTextoOcr(caja, bloque.text)
            }
            onTexto(filtrarTextoEnAreaGuia(bloques, anchoUpright, altoUpright))
        }
        .addOnFailureListener(ejecutorPrincipal) { onFallo() }
        .addOnCompleteListener(ejecutorPrincipal) {
            imagen.close()
        }
}

// Compiladas una sola vez, no dentro de la función -- se llaman en cada
// frame mientras la cámara escanea (ver nota equivalente en
// LectorDocumentosIdentidad.kt).
private val REGEX_CARACTERES_NO_CEDULA = Regex("[^0-9\\n -]")
private val REGEX_CEDULA_CON_GUIONES_O_ESPACIOS = Regex("""\b\d[- ]?\d{4}[- ]?\d{4}\b""")
private val REGEX_CEDULA_PEGADA = Regex("""\b\d{9}\b""")

fun extraerCedulaDeTexto(texto: String): String? {
    val normalizado = texto.replace(REGEX_CARACTERES_NO_CEDULA, " ")
    REGEX_CEDULA_CON_GUIONES_O_ESPACIOS
        .find(normalizado)
        ?.let { return it.value.filter(Char::isDigit) }

    return REGEX_CEDULA_PEGADA
        .find(normalizado)
        ?.value
}
