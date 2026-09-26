package com.brisas.controlacceso

import android.util.Size
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
import androidx.compose.foundation.layout.Box
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.google.mlkit.vision.common.InputImage
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@Composable
fun PantallaEscanearCedula(
    modo: ModoEscaneoDocumento = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
    continuo: Boolean = false,
    // Sólo lo usa el escaneo continuo de gafetes (`PantallaActivos`) -- las
    // otras 3 pantallas de escaneo se quedan con el default (sin tarjeta de
    // resultado) y no cambian en nada. Es una función, no un valor, para
    // leer siempre el estado más reciente del llamador (`ViewModel.mensaje`)
    // en el instante en que cada escaneo termina, sin depender de que
    // Compose ya haya recompuesto con el valor nuevo -- ver el doc-comment
    // en `VistaCamaraCedula` donde se llama.
    resultadoUltimoEscaneo: () -> Pair<String, Boolean>? = { null },
    onDocumentoDetectado: suspend (DocumentoDetectado) -> Unit,
    onCerrar: () -> Unit,
) {
    // Antes el mensaje de permiso era fijo ("...para escanear cédulas"),
    // sin importar el modo -- pedía cédulas incluso escaneando un gafete
    // (hallazgo 2026-09-20, al unificar las 4 pantallas de escaneo).
    val mensajePermiso = when (modo) {
        ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA -> "Se necesita permiso de cámara para escanear cédulas."
        ModoEscaneoDocumento.GAFETE_CONTRATISTA -> "Se necesita permiso de cámara para escanear gafetes."
    }
    EscanerConPermisoCamara(mensajePermiso = mensajePermiso, onCerrar = onCerrar) {
        VistaCamaraCedula(
            modo = modo,
            continuo = continuo,
            resultadoUltimoEscaneo = resultadoUltimoEscaneo,
            onDocumentoDetectado = onDocumentoDetectado,
            onCerrar = onCerrar,
        )
    }
}

@Composable
private fun VistaCamaraCedula(
    modo: ModoEscaneoDocumento,
    continuo: Boolean,
    resultadoUltimoEscaneo: () -> Pair<String, Boolean>?,
    onDocumentoDetectado: suspend (DocumentoDetectado) -> Unit,
    onCerrar: () -> Unit,
) {
    val contexto = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val alcance = rememberCoroutineScope()
    val onDocumentoActual by rememberUpdatedState(onDocumentoDetectado)
    val obtenerResultadoActual by rememberUpdatedState(resultadoUltimoEscaneo)
    // `Vibrator`/`VibrationEffect` directo (`vibrarConfirmacion`/
    // `vibrarError` en `EscaneoCompartido.kt`), no la háptica semántica de
    // Compose -- `HapticFeedbackType.Confirm` resultó no vibrar en un
    // Samsung real con "Interacciones táctiles" activado (hallazgo
    // 2026-09-20), y de cualquier forma esa API no deja elegir amplitud ni
    // patrón para poder distinguir éxito de error.
    // MV-10 (auditoría 2026-09-24): agrupa lo que antes eran 9
    // declaraciones + un DisposableEffect idénticos a las otras 3
    // pantallas de escaneo -- ver EstadoCamaraOcr.kt.
    val camara = rememberEstadoCamaraOcr(contexto)
    // Último punto suelto de MV-07 (auditoría de rendimiento 2026-09-25) --
    // una instancia por apertura de pantalla, igual que `camara`, para
    // reusar los buffers de un frame al siguiente en vez de asignarlos
    // desde cero cada vez. Ver el doc-comment de `BuffersOcrReutilizables`.
    val buffersOcr = remember { BuffersOcrReutilizables() }
    var ultimoMensaje by remember { mutableStateOf(mensajeInicialEscaneo(modo)) }
    var estado by remember { mutableStateOf(EstadoEscaneo.BUSCANDO) }
    var vencido by remember { mutableStateOf(false) }
    // Resultado real de la última mutación (nombre en éxito, motivo en
    // fallo), no sólo "se leyó el gafete" -- ver `resultadoUltimoEscaneo`.
    // `null` mientras no hay nada que mostrar todavía o el llamador no usa
    // esta función (las otras 3 pantallas de escaneo se quedan con el
    // mensaje de siempre).
    var resultadoMostrado by remember { mutableStateOf<Pair<String, Boolean>?>(null) }
    // Una instancia por apertura de pantalla -- lleva el conteo de frames
    // consistentes del debounce (ver EstabilizadorLectura), no debe
    // compartirse entre sesiones de escaneo distintas.
    val estabilizador = remember(modo) { EstabilizadorLectura(modo = modo) }
    var ultimoValorContinuo by remember { mutableStateOf<String?>(null) }
    var framesSinUltimoValor by remember { mutableStateOf(0) }
    // Colores de estado compartidos por las 4 pantallas de escaneo (ver
    // `EscaneoCompartido.kt`) -- fijos, no dependientes del tema
    // (Classic/Brisas/Negro): acá el color comunica significado
    // (buscando/inválido/confirmado/vencido), y ese significado debe leerse
    // igual sin importar qué tema tenga activo quien opera.
    val colorMarco = when {
        estado == EstadoEscaneo.CONFIRMADO && vencido -> ColorEscaneoVencido
        estado == EstadoEscaneo.CONFIRMADO -> ColorEscaneoConfirmado
        estado == EstadoEscaneo.INVALIDO -> ColorEscaneoInvalido
        else -> ColorEscaneoBuscando
    }

    Box(modifier = Modifier.fillMaxSize()) {
        AndroidView(
            factory = { ctx ->
                val previewView = PreviewView(ctx).apply {
                    scaleType = PreviewView.ScaleType.FILL_CENTER
                }
                val onResultado: (ResultadoEstabilizacion) -> Unit = onResultado@{ resultado ->
                        if (!camara.sesionActiva.get()) return@onResultado
                        if (continuo && resultado.estado != EstadoEscaneo.CONFIRMADO) {
                            framesSinUltimoValor++
                            if (framesSinUltimoValor >= FRAMES_AUSENCIA_PARA_REPETIR) {
                                ultimoValorContinuo = null
                                framesSinUltimoValor = 0
                            }
                        }
                        // Vibración de error al ENTRAR a inválido, no en
                        // cada frame que se queda ahí -- pedido explícito
                        // del usuario 2026-09-20 ("si el documento no es
                        // correcto que vibre más"), sin repetir el golpe
                        // mientras la persona sigue apuntando mal.
                        if (resultado.estado == EstadoEscaneo.INVALIDO && estado != EstadoEscaneo.INVALIDO) {
                            vibrarError(contexto)
                        }
                        estado = resultado.estado
                        ultimoMensaje = resultado.mensaje
                        vencido = resultado.vencido
                        val documento = resultado.documento
                        if (resultado.estado == EstadoEscaneo.CONFIRMADO && documento != null) {
                            if (camara.detectada.compareAndSet(false, true)) {
                                val valor = documento.textoBusqueda ?: documento.numeroDocumento
                                val repetidoContinuo = continuo &&
                                    valor == ultimoValorContinuo
                                if (repetidoContinuo) {
                                    camara.detectada.set(false)
                                    estabilizador.reiniciar()
                                } else {
                                    ultimoValorContinuo = valor
                                    framesSinUltimoValor = 0
                                    // En modo continuo el aviso espera al
                                    // resultado real de la mutación (ver más
                                    // abajo) -- pedido explícito del usuario
                                    // 2026-09-20: escanear un gafete que ya
                                    // tenía salida registrada vibraba/sonaba
                                    // exactamente igual que uno que sí salió,
                                    // porque esto disparaba apenas se leía el
                                    // texto, antes de saber si la salida se
                                    // pudo registrar. Sin modo continuo no hay
                                    // "resultado" que esperar -- el llamador
                                    // decide qué hacer con el documento
                                    // después, fuera de esta pantalla.
                                    if (!continuo) {
                                        vibrarConfirmacion(contexto)
                                        reproducirSonidoConfirmacion()
                                    }
                                    camara.trabajoResultado?.cancel()
                                    camara.trabajoResultado = alcance.launch {
                                        // Antes acá se esperaba
                                        // DEMORA_AVISO_VENCIDO_MS si el
                                        // documento estaba vencido, para
                                        // que el operador alcanzara a leer
                                        // el aviso antes de seguir -- se
                                        // saca (pedido explícito del
                                        // usuario, 2026-09-26): la pausa no
                                        // bloqueaba nada de verdad, el
                                        // ingreso se otorgaba igual apenas
                                        // pasaba el segundo. Un mecanismo
                                        // que sí bloquee de verdad (no sólo
                                        // avisar y dejar pasar) queda para
                                        // otra pasada.
                                        if (!camara.sesionActiva.get()) return@launch
                                        // `onDocumentoActual` es suspend: para el
                                        // caso de gafetes ya espera a que la
                                        // mutación en Rust termine (ver
                                        // `ActivosViewModel.registrarSalidaPorGafeteEscaneado`),
                                        // así que al volver de acá el resultado
                                        // que devuelve `obtenerResultadoActual`
                                        // ya es el de ESTE escaneo, no el
                                        // anterior -- se lee directo (una
                                        // función, no un valor recompuesto) para
                                        // no depender de que Compose ya haya
                                        // vuelto a dibujar con el estado nuevo.
                                        onDocumentoActual(documento)
                                        if (continuo && camara.sesionActiva.get()) {
                                            val resultado = obtenerResultadoActual()
                                            if (resultado != null) {
                                                resultadoMostrado = resultado
                                                if (resultado.second) {
                                                    vibrarError(contexto)
                                                } else {
                                                    vibrarConfirmacion(contexto)
                                                    reproducirSonidoConfirmacion()
                                                }
                                            } else {
                                                ultimoMensaje = mensajeProcesadoContinuo(modo, valor)
                                            }
                                            delay(DEMORA_REARMAR_ESCANEO_CONTINUO_MS)
                                            if (camara.sesionActiva.get()) {
                                                camara.detectada.set(false)
                                                estabilizador.reiniciar()
                                                estado = EstadoEscaneo.BUSCANDO
                                                vencido = false
                                                ultimoMensaje = mensajeInicialEscaneo(modo)
                                                resultadoMostrado = null
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                val onFallo: () -> Unit = {
                    if (camara.sesionActiva.get()) {
                        estado = EstadoEscaneo.BUSCANDO
                        vencido = false
                        ultimoMensaje = MENSAJE_FALLO_LECTURA_OCR
                    }
                }
                val analisis = construirAnalizadorOcr(
                    ejecutorAnalisis = camara.ejecutor,
                    detectada = camara.detectada,
                    sesionActiva = camara.sesionActiva,
                ) { imagen ->
                    analizarCedula(
                        imagen = imagen,
                        recognizer = camara.recognizer,
                        ejecutorPrincipal = camara.ejecutorPrincipal,
                        sesionActiva = camara.sesionActiva,
                        buffersOcr = buffersOcr,
                        onTexto = { texto -> onResultado(estabilizador.procesarFrame(texto)) },
                        onFallo = onFallo,
                    )
                }
                camara.analisisCamara = analisis
                iniciarCamara(
                    ctx = ctx,
                    previewView = previewView,
                    lifecycleOwner = lifecycleOwner,
                    analisis = analisis,
                    sesionActiva = camara.sesionActiva,
                    onCameraProviderListo = { proveedor, preview ->
                        camara.cameraProvider = proveedor
                        camara.vistaPreviaCamara = preview
                    },
                    onFallo = { mensaje ->
                        if (camara.sesionActiva.get()) {
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
        // Con resultado real (éxito/fallo de la mutación, no sólo "se leyó
        // el texto"), el mismo mensaje se pinta verde/rojo en vez de negro
        // neutro -- pedido explícito del usuario 2026-09-20: antes, en
        // escaneo continuo, el único aviso de que algo salió mal era el
        // recuadro cambiando de color, fácil de no notar mientras se sigue
        // apuntando la cámara al siguiente gafete.
        val resultado = resultadoMostrado
        Text(
            resultado?.first ?: ultimoMensaje,
            color = Color.White,
            style = MaterialTheme.typography.bodyLarge,
            modifier = Modifier
                .fillMaxWidth()
                .align(Alignment.TopCenter)
                .padding(top = 16.dp, start = 16.dp, end = 72.dp)
                .background(
                    when {
                        resultado == null -> Color.Black.copy(alpha = 0.78f)
                        resultado.second -> ColorEscaneoInvalido.copy(alpha = 0.85f)
                        else -> ColorEscaneoConfirmado.copy(alpha = 0.85f)
                    },
                    FormaCampoBrisas,
                )
                .padding(12.dp),
        )
        // Círculo flotante en vez del texto "Cancelar" que vivía debajo del
        // mensaje -- pedido explícito del usuario (2026-09-19): la posición
        // esquina-superior es la convención de cualquier pantalla de cámara
        // (cerrar sin competir visualmente con el mensaje de estado, y sin
        // el `padding(end = 72.dp)` de arriba el mensaje se le metía debajo).
        // Compartido (`ControlesBrisas.kt`) -- mismo botón en las 4 pantallas
        // de escaneo (Cédula, Carnet KOF, Vehículo/Ruta, Comprobante).
        BotonCerrarCamara(onClick = onCerrar, modifier = Modifier.align(Alignment.TopEnd).padding(16.dp))
    }
}

/// Arma el caso de uso de análisis de ML Kit -- resolución fija y descarte
/// temprano de frames una vez ya se detectó un documento -- compartido por
/// las 4 pantallas de escaneo (Cédula/Gafete, Carnet KOF, Vehículo/Ruta,
/// Comprobante de Ruta). Antes cada pantalla salvo Cédula traía su propia
/// copia textual de este `ImageAnalysis.Builder()` (hallazgo 2026-09-25,
/// mismo riesgo de desincronización que ya se había resuelto para
/// [iniciarCamara]/[analizarCedula]): separado de [iniciarCamara] para que
/// cada función tenga una sola responsabilidad, esta arma "qué se
/// analiza", la otra "cómo se conecta a la cámara física".
///
/// `onFrameActivo` recibe el frame ya filtrado (sesión viva, documento aún
/// no detectado) -- cada pantalla decide ahí cómo llamar a [analizarCedula]
/// con su propio `onTexto`/`onFallo`/`region`, sin que este helper necesite
/// saber nada de estabilizadores ni de qué tipo de documento se busca.
fun construirAnalizadorOcr(
    ejecutorAnalisis: java.util.concurrent.Executor,
    detectada: AtomicBoolean,
    sesionActiva: AtomicBoolean,
    onFrameActivo: (ImageProxy) -> Unit,
): ImageAnalysis {
    // Optimización #2 del relevamiento de rendimiento de cámara
    // (2026-09-25, pospuesta a propósito en el commit c0dad75 para aislar
    // su efecto del resto): antes de esto, con STRATEGY_KEEP_ONLY_LATEST +
    // un solo hilo, se procesaba cada frame que CameraX llegara a entregar
    // -- en un sensor típico de 30fps eso es un `recortarParaOcr` (NV21 ->
    // ARGB + rotar + recortar, todo en CPU) hasta 30 veces por segundo,
    // muchísimo más seguido de lo que ML Kit necesita para leer texto
    // estático. `ultimoFrameProcesadoMs` vive en el closure del analyzer,
    // no en un campo de la pantalla: una sola instancia de
    // `ImageAnalysis`/analyzer por apertura de cámara, y `setAnalyzer` ya
    // garantiza que un único hilo (`ejecutorAnalisis`) llama a este lambda
    // de a un frame por vez, así que un `var` simple alcanza sin
    // sincronización extra.
    //
    // `SystemClock.elapsedRealtime()`, no `System.currentTimeMillis()`:
    // monotónico, no se mueve si cambia la hora del sistema (poco probable
    // acá, pero es el reloj correcto para medir intervalos, no instantes).
    var ultimoFrameProcesadoMs = 0L
    return ImageAnalysis.Builder()
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
                val ahora = android.os.SystemClock.elapsedRealtime()
                if (ahora - ultimoFrameProcesadoMs < INTERVALO_MINIMO_ENTRE_FRAMES_MS) {
                    imagen.close()
                    return@setAnalyzer
                }
                ultimoFrameProcesadoMs = ahora
                onFrameActivo(imagen)
            }
        }
}

/// Conecta el preview y el análisis a la cámara física una vez que
/// `ProcessCameraProvider` está listo, y arranca el enfoque continuo --
/// todo lo que depende de esa espera asíncrona vive acá, separado de cómo
/// se arma el analizador ([construirAnalizadorOcr]).
fun iniciarCamara(
    ctx: android.content.Context,
    previewView: PreviewView,
    lifecycleOwner: androidx.lifecycle.LifecycleOwner,
    analisis: ImageAnalysis,
    sesionActiva: AtomicBoolean,
    onCameraProviderListo: (ProcessCameraProvider, Preview) -> Unit,
    onFallo: (String) -> Unit,
) {
    val cameraProviderFuture = ProcessCameraProvider.getInstance(ctx)
    cameraProviderFuture.addListener(
        {
            if (!sesionActiva.get()) return@addListener
            try {
                val proveedor = cameraProviderFuture.get()
                if (!sesionActiva.get()) return@addListener
                val preview = Preview.Builder().build().also {
                    it.surfaceProvider = previewView.surfaceProvider
                }
                onCameraProviderListo(proveedor, preview)
            // `previewView.viewPort` NO recorta el búfer de `analisis` --
            // para `ImageAnalysis`, CameraX sólo usa el `ViewPort` para
            // calcular `ImageProxy.getCropRect()` (comentario corregido,
            // MV-08 de la auditoría 2026-09-24: la versión anterior decía
            // que esto "ataba el recorte", pero nada leía `cropRect`
            // todavía). `setViewPort` sigue haciendo falta igual -- sin él,
            // `cropRect` queda como el frame completo del sensor, que casi
            // nunca tiene la misma proporción que lo que la vista previa en
            // verdad muestra con FILL_CENTER. `recortarParaOcr` (llamado
            // desde `analizarCedula`) es quien de verdad lee `cropRect` y
            // recorta con eso antes de aplicar el recuadro guía -- ver su
            // doc-comment.
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
// Subido de 900ms -- con el ciclo de salida por gafete ya sin botón de
// confirmar (pedido explícito del usuario 2026-09-20), el mensaje de
// resultado (nombre en verde, motivo en rojo) apenas alcanzaba a leerse
// antes de que la cámara se rearmara para el siguiente. Sigue siendo
// bastante más rápido que tener que confirmar a mano.
private const val DEMORA_REARMAR_ESCANEO_CONTINUO_MS = 1600L
private const val FRAMES_AUSENCIA_PARA_REPETIR = 3

// Ver el doc-comment de `construirAnalizadorOcr` (optimización #2). 150ms
// = ~6-7 frames/s procesados como techo, muy por encima de lo que
// `EstabilizadorLectura` necesita (2-3 lecturas consistentes para
// confirmar) y bien por debajo de lo que se sentiría como lag al apuntar
// la cámara -- el sensor sigue entregando a su fps normal, esto sólo
// descarta los frames de más entre medio antes de gastar CPU en ellos.
private const val INTERVALO_MINIMO_ENTRE_FRAMES_MS = 150L

/// Compartido por las 4 pantallas de escaneo -- antes cada una tenía su
/// propia copia textual idéntica (hallazgo 2026-09-19, riesgo de
/// desincronizarse si alguien edita una sin las otras 3). Sin `private`:
/// vive acá porque [analizarCedula]/[iniciarCamara] (el resto de lo
/// compartido) también viven en este archivo.
const val MENSAJE_FALLO_LECTURA_OCR = "No se pudo leer el texto. Intente acercar."

// El reverso (con el MRZ -- las líneas de texto tipo código de barras) trae
// nombre Y cédula en un solo escaneo con checksum verificado; el frente
// (con la foto) sólo trae el número. Guiar hacia el reverso desde el
// mensaje inicial evita que quien opera tenga que enterarse por su cuenta
// (ver el aviso en `EstabilizadorLectura.mensajeDeConfirmacion` para cuando
// igual termina mostrando el frente).
private fun mensajeInicialEscaneo(modo: ModoEscaneoDocumento): String =
    when (modo) {
        ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA -> "Muéstreme el reverso de la cédula"
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
// No `private` -- [analizarCedula] no conoce nada de cédulas ni de
// documentos de identidad (sólo entrega texto crudo de ML Kit), así que
// otros perfiles de OCR aislados (ver LectorComprobanteRuta.kt /
// PantallaEscanearComprobanteRuta.kt) la reusan en vez de duplicar el
// manejo de `ImageProxy`/`InputImage`/hilos. Mismo motivo para
// [iniciarCamara] más arriba.
@androidx.annotation.OptIn(ExperimentalGetImage::class)
fun analizarCedula(
    imagen: ImageProxy,
    recognizer: com.google.mlkit.vision.text.TextRecognizer,
    ejecutorPrincipal: java.util.concurrent.Executor,
    sesionActiva: AtomicBoolean,
    // `null` (default) preserva el comportamiento de siempre -- asignar los
    // buffers desde cero por frame. Pasarlo es lo que cierra el último
    // punto suelto de MV-07 (ver `BuffersOcrReutilizables`).
    buffersOcr: BuffersOcrReutilizables? = null,
    onTexto: (String) -> Unit,
    onFallo: () -> Unit,
    // Angosta (proporción de tarjeta) por defecto. `null` desactiva el
    // recorte por completo (frame entero, como antes de este cambio) --
    // el comprobante de carga de ruta lo usa así: es un papel mucho más
    // grande que una tarjeta y, sin datos reales todavía de qué región
    // exacta conviene recortar, adivinar mal significaba dejar de leer
    // CUALQUIER campo (hallazgo 2026-09-20) en vez de sólo leer peor.
    region: RegionGuiaOcr? = RegionGuiaOcr.TARJETA_ID,
) {
    val mediaImage = imagen.image
    if (mediaImage == null) {
        imagen.close()
        return
    }
    // Optimización #4 del relevamiento de rendimiento de cámara
    // (2026-09-25, pospuesta a propósito junto con #2 en el commit
    // c0dad75): `construirAnalizadorOcr` ya filtra por `sesionActiva` antes
    // de llegar acá, pero ese chequeo pasa ANTES del trabajo pesado de esta
    // función (recortar = NV21->ARGB + rotar + recortar Bitmap), no
    // después. Con `setAnalyzer` en un único hilo casi nunca hay hueco
    // entre ambos chequeos, pero repetirlo acá, justo antes de
    // `recortarParaOcr`, es gratis (una lectura de `AtomicBoolean`) y cierra
    // la ventana por completo: si la pantalla se cerró en el instante entre
    // ambos chequeos, no se gasta CPU recortando un frame cuyo resultado
    // nadie va a usar.
    if (!sesionActiva.get()) {
        imagen.close()
        return
    }
    val rotacion = imagen.imageInfo.rotationDegrees
    // MV-08 (auditoría 2026-09-24): `imagen.cropRect` es lo único que
    // CameraX de verdad ajusta a partir del `ViewPort` (`setViewPort` en
    // `iniciarCamara`) -- para `ImageAnalysis` el búfer en sí NUNCA se
    // recorta, sólo se informa qué porción de él corresponde a lo visible.
    // Se lee acá, sobre el `ImageProxy`, porque `mediaImage`
    // (`android.media.Image`) no expone esta información. Antes de este
    // fix se ignoraba por completo y `recortarParaOcr` trabajaba siempre
    // sobre el frame entero del sensor.
    val cropRect = imagen.cropRect
    // Recorta al mismo recuadro que ve la persona en pantalla antes de
    // mandarle el frame a ML Kit -- pedido explícito del usuario 2026-09-20
    // para que el reconocimiento sea más rápido (menos píxeles) y más
    // preciso (el texto de interés ocupa más del cuadro, sin ruido de fondo
    // compitiendo), tal como recomienda la guía oficial de ML Kit. Con
    // `recortarParaOcr` devolviendo `null` (formato inesperado, plano
    // corrupto, lo que sea) se cae al frame completo de siempre -- nunca
    // debe romper el escaneo por un recorte que salió mal.
    val input = region?.let { recortarParaOcr(mediaImage, cropRect, rotacion, it, buffersOcr) }
        ?: InputImage.fromMediaImage(mediaImage, rotacion)
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

/// Recorta el frame de la cámara al mismo recuadro que dibuja
/// `MarcoGuiaCedula` (`RegionGuiaOcr`, misma proporción/posición) antes de
/// mandarlo a ML Kit. `null` si algo no sale como se espera -- el llamador
/// cae de vuelta al frame completo, nunca debe romper el escaneo.
///
/// Por qué rota A BITMAP COMPLETO primero y recién ahí recorta, en vez de
/// calcular el recorte directo sobre el buffer crudo (que ahorraría
/// armar el bitmap completo): el recuadro que ve la persona está expresado
/// en coordenadas YA ROTADAS (como la pantalla, vertical), mientras que
/// `imagen`/sus planos vienen en la orientación nativa del sensor (normal
/// que la cámara trasera entregue esto en apaisado incluso con el teléfono
/// en vertical). Traducir el recuadro vertical a coordenadas del sensor sin
/// rotar exige invertir a mano el giro de 90°/270° que aplica la cámara --
/// exactamente el tipo de mapeo de coordenadas que ya salió mal una vez en
/// este archivo (ver el comentario de `analizarCedula`, sección 0.6 del
/// plan, sobre por qué el recuadro de guía es deliberadamente estático).
/// Rotar primero devuelve un bitmap donde "arriba/ancho/alto" ya significan
/// lo mismo que en pantalla, así que el recorte usa la misma aritmética que
/// `MarcoGuiaCedula` sin ningún signo que invertir.
///
/// El bitmap completo se arma con [convertirNv21AArgb] directo, sin pasar
/// por JPEG (auditoría de rendimiento 2026-09-25: `YuvImage.compressToJpeg`
/// + `BitmapFactory.decodeByteArray` era el costo de CPU más alto por
/// frame de las 4 pantallas de escaneo, y de paso comprimía con pérdida una
/// imagen que nunca se guarda ni se muestra). Cada bitmap intermedio que ya
/// no hace falta se recicla apenas se arma el siguiente -- el único que
/// queda vivo al salir es `bitmapRecortado`, porque `recognizer.process`
/// todavía lo necesita después de este `return`.
///
/// `cropRect` (MV-08, auditoría 2026-09-24): antes de rotar, se recorta al
/// rectángulo que CameraX calculó a partir del `ViewPort` (o al frame
/// completo si todavía es `null`/no cambió nada, ej. antes de que la vista
/// tenga tamaño) -- SIN esto, el bitmap de partida es el frame entero del
/// sensor, que casi nunca tiene la misma proporción que lo que en verdad
/// se ve en pantalla con `FILL_CENTER`, así que el recuadro guía terminaba
/// analizando una región más ancha que la visible (podía leer un documento
/// fuera del marco). `cropRect` viene en coordenadas del búfer SIN rotar
/// (mismo espacio que `imagen.width`/`imagen.height`), así que este recorte
/// va antes de rotar, no después.
@androidx.annotation.OptIn(ExperimentalGetImage::class)
private fun recortarParaOcr(
    imagen: android.media.Image,
    cropRect: android.graphics.Rect,
    rotacionGrados: Int,
    region: RegionGuiaOcr,
    // `null` (default) preserva el comportamiento de siempre. Ver
    // `BuffersOcrReutilizables` -- último punto suelto de MV-07.
    buffersOcr: BuffersOcrReutilizables? = null,
): InputImage? {
    if (imagen.format != android.graphics.ImageFormat.YUV_420_888) return null
    val planos = imagen.planes
    if (planos.size < 3) return null
    return try {
        val yPlano = planos[0]
        val uPlano = planos[1]
        val vPlano = planos[2]
        val tamanoY = yPlano.buffer.remaining()
        val tamanoU = uPlano.buffer.remaining()
        val tamanoV = vPlano.buffer.remaining()
        val yBytes = (buffersOcr?.yBytes(tamanoY) ?: ByteArray(tamanoY)).also { yPlano.buffer.get(it) }
        val uBytes = (buffersOcr?.uBytes(tamanoU) ?: ByteArray(tamanoU)).also { uPlano.buffer.get(it) }
        val vBytes = (buffersOcr?.vBytes(tamanoV) ?: ByteArray(tamanoV)).also { vPlano.buffer.get(it) }
        val nv21 = construirNv21(
            ancho = imagen.width,
            alto = imagen.height,
            y = yBytes,
            yRowStride = yPlano.rowStride,
            u = uBytes,
            v = vBytes,
            uvRowStride = uPlano.rowStride,
            uvPixelStride = uPlano.pixelStride,
            destino = buffersOcr?.nv21((imagen.width * imagen.height) + (imagen.width * imagen.height) / 2),
        )
        val pixeles = convertirNv21AArgb(
            nv21, imagen.width, imagen.height,
            destino = buffersOcr?.pixeles(imagen.width * imagen.height),
        )
        val bitmapCompleto = android.graphics.Bitmap.createBitmap(
            pixeles, imagen.width, imagen.height, android.graphics.Bitmap.Config.ARGB_8888,
        )
        // MV-08: recorta al `cropRect` del ViewPort ANTES de rotar --
        // `cropRect` está en el mismo espacio sin rotar que
        // `imagen.width`/`imagen.height`. `intersect` contra el bitmap
        // completo por seguridad (un `cropRect` corrido o más grande que el
        // frame -- no debería pasar, pero `createBitmap` tira si el
        // rectángulo se sale) cae de vuelta al frame entero en vez de
        // fallar el escaneo.
        val cropSeguro = android.graphics.Rect(cropRect)
        if (!cropSeguro.intersect(0, 0, bitmapCompleto.width, bitmapCompleto.height)) {
            cropSeguro.set(0, 0, bitmapCompleto.width, bitmapCompleto.height)
        }
        val bitmapAlViewport = if (cropSeguro.width() == bitmapCompleto.width && cropSeguro.height() == bitmapCompleto.height) {
            bitmapCompleto
        } else {
            android.graphics.Bitmap.createBitmap(
                bitmapCompleto, cropSeguro.left, cropSeguro.top, cropSeguro.width(), cropSeguro.height(),
            ).also { if (it !== bitmapCompleto) bitmapCompleto.recycle() }
        }
        val bitmapDerecho = if (rotacionGrados == 0) {
            bitmapAlViewport
        } else {
            val matriz = android.graphics.Matrix().apply { postRotate(rotacionGrados.toFloat()) }
            android.graphics.Bitmap.createBitmap(
                bitmapAlViewport, 0, 0, bitmapAlViewport.width, bitmapAlViewport.height, matriz, false,
            ).also { if (it !== bitmapAlViewport) bitmapAlViewport.recycle() }
        }
        val recorte = region.rectanguloEnPixeles(bitmapDerecho.width, bitmapDerecho.height)
        if (recorte.width <= 0 || recorte.height <= 0) return null
        val bitmapRecortado = android.graphics.Bitmap.createBitmap(
            bitmapDerecho, recorte.left, recorte.top, recorte.width, recorte.height,
        ).also { if (it !== bitmapDerecho) bitmapDerecho.recycle() }
        InputImage.fromBitmap(bitmapRecortado, 0)
    } catch (e: Exception) {
        null
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
