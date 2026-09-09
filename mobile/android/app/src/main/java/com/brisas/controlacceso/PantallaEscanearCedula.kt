package com.brisas.controlacceso

import android.Manifest
import android.content.pm.PackageManager
import android.media.AudioManager
import android.media.ToneGenerator
import android.os.Handler
import android.os.Looper
import android.util.Size
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.ExperimentalGetImage
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.core.UseCaseGroup
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.background
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
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@Composable
fun PantallaEscanearCedula(
    modo: ModoEscaneoDocumento = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
    continuo: Boolean = false,
    onDocumentoDetectado: suspend (DocumentoDetectado) -> Unit,
    onCerrar: () -> Unit,
) {
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
        VistaCamaraCedula(
            modo = modo,
            continuo = continuo,
            onDocumentoDetectado = onDocumentoDetectado,
            onCerrar = onCerrar,
        )
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
private fun VistaCamaraCedula(
    modo: ModoEscaneoDocumento,
    continuo: Boolean,
    onDocumentoDetectado: suspend (DocumentoDetectado) -> Unit,
    onCerrar: () -> Unit,
) {
    val contexto = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val alcance = rememberCoroutineScope()
    val onDocumentoActual by rememberUpdatedState(onDocumentoDetectado)
    // Háptica semántica de Compose (`HapticFeedbackType.Confirm`), no
    // `Vibrator`/`VibrationEffect` crudo -- la guía oficial de Android
    // desaconseja `createOneShot`/`createWaveform` para feedback de UI
    // regular ("demasiado fuerte/genérico"); el tipo `Confirm` está pensado
    // exactamente para esto, no requiere permiso VIBRATE, y respeta la
    // intensidad háptica que la persona ya configuró en el sistema en vez
    // de imponer una vibración fija.
    val haptica = LocalHapticFeedback.current
    val ejecutor = remember { Executors.newSingleThreadExecutor() }
    val recognizer = remember { TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS) }
    var ultimoMensaje by remember { mutableStateOf(mensajeInicialEscaneo(modo)) }
    var estado by remember { mutableStateOf(EstadoEscaneo.BUSCANDO) }
    var vencido by remember { mutableStateOf(false) }
    // Una instancia por apertura de pantalla -- lleva el conteo de frames
    // consistentes del debounce (ver EstabilizadorLectura), no debe
    // compartirse entre sesiones de escaneo distintas.
    val estabilizador = remember(modo) { EstabilizadorLectura(modo = modo) }
    var ultimoValorContinuo by remember { mutableStateOf<String?>(null) }
    var framesSinUltimoValor by remember { mutableStateOf(0) }
    // AtomicBoolean, no `mutableStateOf` -- esta bandera se lee en el hilo
    // del analizador de cámara (`ejecutor`) y se escribe desde el hilo
    // principal (callback de ML Kit); un booleano de Compose no garantiza
    // esa visibilidad entre hilos, y además el `compareAndSet` evita que
    // dos frames en vuelo disparen `onDocumentoDetectado` dos veces.
    val detectada = remember { AtomicBoolean(false) }
    // Invalida callbacks de CameraX/ML Kit que terminen después de salir de
    // esta composición. Cerrar el recognizer no garantiza que un Task que ya
    // estaba en vuelo deje de entregar su listener.
    val sesionActiva = remember { AtomicBoolean(true) }
    // Guardado acá para poder desatarlo explícitamente al salir -- `bindToLifecycle`
    // por sí solo no alcanza: en una app de una sola Activity con Compose,
    // `LocalLifecycleOwner` suele ser la Activity, no esta pantalla, así que la
    // cámara no se libera sola al navegar fuera de acá, sólo al morir la Activity.
    // Sin este `unbindAll()` explícito, reabrir el escáner puede encontrar la
    // cámara todavía atada al ciclo de vida anterior.
    var cameraProvider by remember { mutableStateOf<ProcessCameraProvider?>(null) }
    var analisisCamara by remember { mutableStateOf<ImageAnalysis?>(null) }
    var trabajoResultado by remember { mutableStateOf<Job?>(null) }
    // Ver nota en `analizarCedula`: se pasa explícito en vez de dejar que
    // ML Kit use su executor por defecto de forma implícita.
    val ejecutorPrincipal = remember { ContextCompat.getMainExecutor(contexto) }

    DisposableEffect(Unit) {
        sesionActiva.set(true)
        onDispose {
            sesionActiva.set(false)
            detectada.set(true)
            trabajoResultado?.cancel()
            analisisCamara?.clearAnalyzer()
            analisisCamara?.let { cameraProvider?.unbind(it) }
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
                    sesionActiva = sesionActiva,
                    onResultado = { resultado ->
                        if (!sesionActiva.get()) return@construirAnalizadorOcr
                        if (continuo && resultado.estado != EstadoEscaneo.CONFIRMADO) {
                            framesSinUltimoValor++
                            if (framesSinUltimoValor >= FRAMES_AUSENCIA_PARA_REPETIR) {
                                ultimoValorContinuo = null
                                framesSinUltimoValor = 0
                            }
                        }
                        estado = resultado.estado
                        ultimoMensaje = resultado.mensaje
                        vencido = resultado.vencido
                        val documento = resultado.documento
                        if (resultado.estado == EstadoEscaneo.CONFIRMADO && documento != null) {
                            if (detectada.compareAndSet(false, true)) {
                                val valor = documento.textoBusqueda ?: documento.numeroDocumento
                                val repetidoContinuo = continuo &&
                                    valor == ultimoValorContinuo
                                if (repetidoContinuo) {
                                    detectada.set(false)
                                    estabilizador.reiniciar()
                                } else {
                                    ultimoValorContinuo = valor
                                    framesSinUltimoValor = 0
                                    haptica.performHapticFeedback(HapticFeedbackType.Confirm)
                                    reproducirSonidoConfirmacion()
                                    trabajoResultado?.cancel()
                                    trabajoResultado = alcance.launch {
                                        if (!continuo && resultado.vencido) {
                                            delay(DEMORA_AVISO_VENCIDO_MS)
                                        }
                                        if (!sesionActiva.get()) return@launch
                                        onDocumentoActual(documento)
                                        if (continuo && sesionActiva.get()) {
                                            ultimoMensaje = mensajeProcesadoContinuo(modo, valor)
                                            delay(DEMORA_REARMAR_ESCANEO_CONTINUO_MS)
                                            if (sesionActiva.get()) {
                                                detectada.set(false)
                                                estabilizador.reiniciar()
                                                estado = EstadoEscaneo.BUSCANDO
                                                vencido = false
                                                ultimoMensaje = mensajeInicialEscaneo(modo)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    onFallo = {
                        if (!sesionActiva.get()) return@construirAnalizadorOcr
                        estado = EstadoEscaneo.BUSCANDO
                        vencido = false
                        ultimoMensaje = "No se pudo leer el texto. Intente acercar."
                    },
                )
                analisisCamara = analisis
                iniciarCamara(
                    ctx = ctx,
                    previewView = previewView,
                    lifecycleOwner = lifecycleOwner,
                    analisis = analisis,
                    sesionActiva = sesionActiva,
                    onCameraProviderListo = { cameraProvider = it },
                    onFallo = { mensaje ->
                        if (sesionActiva.get()) {
                            estado = EstadoEscaneo.INVALIDO
                            ultimoMensaje = mensaje
                        }
                    },
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
    sesionActiva: AtomicBoolean,
    onResultado: (ResultadoEstabilizacion) -> Unit,
    onFallo: () -> Unit,
): ImageAnalysis =
    ImageAnalysis.Builder()
        .setResolutionSelector(
            ResolutionSelector.Builder()
                .setResolutionStrategy(
                    ResolutionStrategy(
                        Size(1280, 720),
                        ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER,
                    ),
                )
                .build(),
        )
        .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
        .build()
        .also { analisis ->
            analisis.setAnalyzer(ejecutorAnalisis) { imagen ->
                // Ya se detectó un documento y se avisó al llamador --
                // seguir corriendo ML Kit en cada frame mientras la
                // pantalla termina de cerrarse sólo quema CPU sin ganar
                // nada (el resultado ya se usó).
                if (!sesionActiva.get() || detectada.get()) {
                    imagen.close()
                    return@setAnalyzer
                }
                analizarCedula(
                    imagen = imagen,
                    recognizer = recognizer,
                    ejecutorPrincipal = ejecutorPrincipal,
                    sesionActiva = sesionActiva,
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
    sesionActiva: AtomicBoolean,
    onCameraProviderListo: (ProcessCameraProvider) -> Unit,
    onFallo: (String) -> Unit,
) {
    val cameraProviderFuture = ProcessCameraProvider.getInstance(ctx)
    cameraProviderFuture.addListener(
        {
            if (!sesionActiva.get()) return@addListener
            try {
                val proveedor = cameraProviderFuture.get()
                if (!sesionActiva.get()) return@addListener
                onCameraProviderListo(proveedor)
                val preview = Preview.Builder().build().also {
                    it.surfaceProvider = previewView.surfaceProvider
                }
            // `previewView.viewPort` ata el recorte de `analisis` al mismo
            // rectángulo que en verdad se ve en pantalla (la vista previa
            // usa FILL_CENTER, que recorta/escala el frame del sensor a la
            // proporción de la pantalla -- casi nunca la misma proporción
            // que el sensor). Sin esto, `filtrarTextoEnAreaGuia` compara
            // contra las dimensiones crudas del sensor, no contra lo que la
            // persona realmente ve dentro del recuadro guía: el recuadro en
            // pantalla y la zona que de verdad analiza ML Kit terminan
            // siendo rectángulos físicos distintos.
                val grupoUseCases = UseCaseGroup.Builder()
                    .addUseCase(preview)
                    .addUseCase(analisis)
                    .apply { previewView.viewPort?.let { setViewPort(it) } }
                    .build()
                proveedor.bindToLifecycle(
                    lifecycleOwner,
                    CameraSelector.DEFAULT_BACK_CAMERA,
                    grupoUseCases,
                )
            } catch (_: Exception) {
                if (sesionActiva.get()) onFallo("No se pudo iniciar la cámara")
            }
        },
        ContextCompat.getMainExecutor(ctx),
    )
}

// Hubo acá un empujón manual de autofocus al centro (`FocusMeteringAction`)
// para ayudar a enfocar de cerca (10-15cm) al arrancar -- ver plan, sección
// 0.6. Se sacó por completo: el autofocus continuo por defecto de CameraX
// (sin ningún `FocusMeteringAction` custom) es exactamente lo que tenía la
// app cuando el reconocimiento era instantáneo, antes de que se agregara
// este empujón. Una variante intermedia con `disableAutoCancel()` resultó
// ser el bug real detrás de "hay que sostenerlo en un ángulo muy
// específico": esa llamada no da "enfoque continuo" pese a lo que decía un
// comentario anterior acá -- bloquea el foco para siempre en lo que la
// cámara haya visto en el instante en que arrancó, antes de que la persona
// alcance a poner el documento enfrente. Sacar `disableAutoCancel()` mejoró
// las cosas pero seguía sin sentirse tan rápido como el autofocus puramente
// por defecto -- un empujón puntual solo puede sumar latencia sin garantía
// de ayudar, así que se sacó del todo.

/// Sonido corto y discreto de confirmación -- complementa la háptica, no la
/// reemplaza (alguien con el celular en silencio/vibrador no debería
/// quedarse sin ninguna señal, y viceversa). `TONE_PROP_ACK` es
/// literalmente el tono que Android reserva para "confirmación positiva",
/// no un beep genérico. Se reproduce en `STREAM_NOTIFICATION`: ese stream
/// respeta el modo silencioso/No molestar del sistema automáticamente, así
/// que no hace falta consultar `AudioManager.getRingerMode()` a mano --
/// dejar que el sistema decida si corresponde sonar es más confiable que
/// replicar esa lógica acá. Volumen bajo (`MAX_VOLUME` es 100) y duración
/// corta a propósito: nada de un beep de escáner de supermercado.
private fun reproducirSonidoConfirmacion() {
    try {
        val generador = ToneGenerator(AudioManager.STREAM_NOTIFICATION, VOLUMEN_SONIDO_CONFIRMACION)
        generador.startTone(ToneGenerator.TONE_PROP_ACK, DURACION_SONIDO_CONFIRMACION_MS)
        // ToneGenerator reserva un recurso nativo de audio hasta `release()` --
        // sin esto se queda tomado el resto de la vida del proceso. El
        // delay deja que el tono realmente termine de sonar antes de soltarlo.
        Handler(Looper.getMainLooper()).postDelayed(generador::release, DURACION_SONIDO_CONFIRMACION_MS + 50L)
    } catch (e: RuntimeException) {
        // El constructor de ToneGenerator puede fallar si el dispositivo no
        // tiene el recurso de audio disponible en ese momento -- el sonido
        // es un complemento, nunca debe tumbar el flujo de escaneo por esto.
    }
}

private const val VOLUMEN_SONIDO_CONFIRMACION = 40 // sobre 100 -- sutil, no un beep de caja registradora
private const val DURACION_SONIDO_CONFIRMACION_MS = 100
private const val DEMORA_AVISO_VENCIDO_MS = 1200L
private const val DEMORA_REARMAR_ESCANEO_CONTINUO_MS = 900L
private const val FRAMES_AUSENCIA_PARA_REPETIR = 3

private fun mensajeInicialEscaneo(modo: ModoEscaneoDocumento): String =
    when (modo) {
        ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA -> "Apunte al documento"
        ModoEscaneoDocumento.GAFETE_CONTRATISTA -> "Apunte al gafete"
    }

private fun mensajeProcesadoContinuo(modo: ModoEscaneoDocumento, valor: String): String =
    when (modo) {
        ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA -> "Documento $valor procesado"
        ModoEscaneoDocumento.GAFETE_CONTRATISTA -> "Gafete $valor procesado"
    }

/// Sólo entrega a ML Kit y devuelve el texto reconocido -- la clasificación
/// de tipo de documento, extracción de campos y decisión de aceptar o no la
/// lectura viven en [EstabilizadorLectura], no acá (separar esto evita que
/// esta función termine "sabiendo" de cédulas/DIMEX/licencias/MRZ).
///
/// Analiza el frame completo, sin recortar al recuadro guía -- el plan
/// (sección 9) proponía filtrar los `TextBlock` de ML Kit por su
/// `boundingBox` contra el recuadro para "no distraer" a ML Kit con texto de
/// fondo, pero en la práctica volvía el escaneo mucho más incómodo (había
/// que encuadrar el documento con precisión milimétrica para que
/// reconociera algo, contra el reconocimiento casi instantáneo de antes) sin
/// aportar la velocidad prometida -- ML Kit igual procesa el frame entero
/// antes de filtrar, el recorte solo descartaba resultados después. El
/// recuadro (`MarcoGuiaCedula`) queda como guía visual, no como filtro.
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
    sesionActiva: AtomicBoolean,
    onTexto: (String) -> Unit,
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
            if (sesionActiva.get()) {
                onTexto(resultado.text)
            }
        }
        .addOnFailureListener(ejecutorPrincipal) { if (sesionActiva.get()) onFallo() }
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
private val REGEX_LINEA_NUMERO_AJENO = Regex(
    """\bEXPEDIENTE\s*(?:N[O°º.]*)?""",
    RegexOption.IGNORE_CASE,
)

fun extraerCedulaDeTexto(texto: String): String? {
    val lineasCandidatas = texto
        .lines()
        .filterNot { REGEX_LINEA_NUMERO_AJENO.containsMatchIn(it) }

    for (linea in lineasCandidatas) {
        val normalizada = linea.replace(REGEX_CARACTERES_NO_CEDULA, " ")
        REGEX_CEDULA_CON_GUIONES_O_ESPACIOS
            .find(normalizada)
            ?.let { return it.value.filter(Char::isDigit) }
    }

    for (linea in lineasCandidatas) {
        val normalizada = linea.replace(REGEX_CARACTERES_NO_CEDULA, " ")
        REGEX_CEDULA_PEGADA
            .find(normalizada)
            ?.let { return it.value }
    }

    return null
}
