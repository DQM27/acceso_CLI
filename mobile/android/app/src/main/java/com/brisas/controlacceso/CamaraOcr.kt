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
import androidx.core.content.ContextCompat
import com.google.mlkit.vision.common.InputImage
import java.util.concurrent.atomic.AtomicBoolean

/// Plomería de cámara + ML Kit compartida por las pantallas de escaneo
/// (sacada de `PantallaEscanearCedula.kt`, punto M4 de la auditoría
/// móvil): analizador de CameraX con límite de frames, arranque de la
/// cámara, recorte al recuadro guía sobre los planos YUV y entrega a ML
/// Kit. Sin nada de clasificación de documentos -- eso vive en
/// `LectorDocumentosIdentidad.kt` / `EstabilizadorLectura.kt`.

/// Arma el caso de uso de análisis de ML Kit -- resolución fija y descarte
/// temprano de frames una vez ya se detectó un documento -- compartido por
/// las 4 pantallas de escaneo (Cédula/Gafete, Carnet KOF, Vehículo/Ruta,
/// Comprobante de Ruta). Antes cada pantalla salvo Cédula traía su propia
/// copia textual de este `ImageAnalysis.Builder()` (hallazgo 2026-09-25,
/// mismo riesgo de desincronización que ya se había resuelto para
/// [iniciarCamara]/[analizarFrameOcr]): separado de [iniciarCamara] para que
/// cada función tenga una sola responsabilidad, esta arma "qué se
/// analiza", la otra "cómo se conecta a la cámara física".
///
/// `onFrameActivo` recibe el frame ya filtrado (sesión viva, documento aún
/// no detectado) -- cada pantalla decide ahí cómo llamar a [analizarFrameOcr]
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
    // -- en un sensor típico de 30fps eso es un `recortarParaOcr` (copiar planos
    // + recortar a NV21, todo en CPU) hasta 30 veces por segundo,
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
                        // 1080p: a 720p el MRZ de una cédula dentro del
                        // recuadro guía quedaba con letras de ~17 px de alto,
                        // justo en el mínimo que pide ML Kit (~16 px; ideal
                        // ~24) -- de ahí buena parte de los `<` perdidos y
                        // las confusiones de caracteres. El costo extra ya no
                        // pesa: se recorta sólo el recuadro sobre los planos
                        // crudos (ver `recortarParaOcr`), sin convertir el
                        // frame completo a ARGB.
                        Size(1920, 1080),
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
    // La cámara ya conectada: la usa la linterna (`EstadoCamaraOcr`).
    onCamaraLista: (androidx.camera.core.Camera) -> Unit = {},
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
            // desde `analizarFrameOcr`) es quien de verdad lee `cropRect` y
            // recorta con eso antes de aplicar el recuadro guía -- ver su
            // doc-comment.
                val grupoUseCases = UseCaseGroup.Builder()
                    .addUseCase(preview)
                    .addUseCase(analisis)
                    .apply { previewView.viewPort?.let { setViewPort(it) } }
                    .build()
                val camara = proveedor.bindToLifecycle(
                    lifecycleOwner,
                    CameraSelector.DEFAULT_BACK_CAMERA,
                    grupoUseCases,
                )
                onCamaraLista(camara)
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
/// vive acá porque [analizarFrameOcr]/[iniciarCamara] (el resto de lo
/// compartido) también viven en este archivo.
const val MENSAJE_FALLO_LECTURA_OCR = "No se pudo leer. Acerque el documento y evite reflejos."

/// Analiza un frame: recorta al recuadro, mide su calidad, corre el lector
/// que toque (texto o códigos) y entrega una [LecturaFrame]. No sabe nada
/// de cédulas ni de placas: clasificar y decidir vive en los
/// estabilizadores.
///
/// Etapas, en orden de costo:
/// 1. Recorte sobre los planos YUV ([recortarParaOcr]) -- ML Kit recibe
///    sólo `region` (el recuadro guía, o la banda del MRZ); `null` = frame
///    entero.
/// 2. Calidad ([medirCalidad]/[FiltroCalidad], auditoría OCR 2026-09-28):
///    un frame claramente más borroso que los recientes NO pasa a ML Kit,
///    lo más caro del frame. Los que pasan llevan su peso para la votación.
/// 3. ML Kit: texto, o PDF417 de la cédula anterior cuando `lector` es
///    [LectorFrame.CODIGO] (sólo se entrega si se leyó una cédula válida).
///
/// `onLectura`/`onFallo` corren en el HILO DEL ANALIZADOR, no en el
/// principal: clasificar, votar y cruzar a Rust no debe trabar la
/// pantalla. Es seguro porque CameraX no entrega otro frame hasta que éste
/// se cierra (al completar), así que todo queda serializado en un hilo.
/// Quien llama publica al hilo principal sólo el estado de pantalla (ver
/// [EstadoCamaraOcr.enPrincipal]).
@androidx.annotation.OptIn(ExperimentalGetImage::class)
fun analizarFrameOcr(
    imagen: ImageProxy,
    camara: EstadoCamaraOcr,
    region: RegionRecorte?,
    lector: LectorFrame = LectorFrame.TEXTO,
    onLectura: (LecturaFrame) -> Unit,
    onFallo: () -> Unit,
) {
    val mediaImage = imagen.image
    // Se repite el chequeo de sesión justo antes del trabajo pesado: si la
    // pantalla se cerró entre medio, no se gasta CPU en un frame que nadie
    // va a usar.
    if (mediaImage == null || !camara.sesionActiva.get()) {
        imagen.close()
        return
    }
    val inicio = System.nanoTime()
    val rotacion = imagen.imageInfo.rotationDegrees
    // MV-08 (auditoría 2026-09-24): `cropRect` es lo que CameraX ajusta a
    // partir del `ViewPort` -- la porción del búfer que de verdad se ve en
    // pantalla (el búfer de `ImageAnalysis` nunca se recorta solo).
    val recorte = region?.let { recortarParaOcr(mediaImage, imagen.cropRect, rotacion, it) }
    val calidad = recorte?.let { medirCalidad(it.nv21, it.anchoSensor, it.altoSensor) }
    val decision = if (calidad != null && region != null) {
        camara.filtroCalidad(region).evaluar(calidad)
    } else {
        DecisionCalidad(procesar = true, peso = 1f)
    }
    if (!decision.procesar) {
        camara.metricas.registrarDescarte()
        imagen.close()
        return
    }
    val finRecorte = System.nanoTime()
    // Sin recorte (formato inesperado, plano raro) se lee el frame entero:
    // un recorte fallido nunca debe romper el escaneo.
    val input = recorte?.input ?: InputImage.fromMediaImage(mediaImage, rotacion)
    val regionLeida = if (recorte != null) region else null
    val ejecutor = camara.ejecutorProcesamiento

    val lectorCodigos = camara.lectorCodigos
    if (lector == LectorFrame.CODIGO && lectorCodigos != null) {
        lectorCodigos.process(input).addOnSuccessListener(ejecutor) { codigos ->
            if (!camara.sesionActiva.get()) return@addOnSuccessListener
            camara.metricas.registrarFrame(LectorFrame.CODIGO, finRecorte - inicio, System.nanoTime() - finRecorte)
            val datos = codigos.firstNotNullOfOrNull { codigo -> codigo.rawBytes?.let(::extraerPdf417Cedula) }
            if (datos != null) {
                onLectura(
                    LecturaFrame(LectorFrame.CODIGO, "", datos, calidad, decision.peso, regionLeida, emptyList()),
                )
            }
        }.cerrarAlTerminar(imagen, camara, onFallo)
    } else {
        camara.recognizer.process(input).addOnSuccessListener(ejecutor) { resultado ->
            if (!camara.sesionActiva.get()) return@addOnSuccessListener
            camara.metricas.registrarFrame(LectorFrame.TEXTO, finRecorte - inicio, System.nanoTime() - finRecorte)
            val lineasMrz = if (recorte == null) {
                emptyList()
            } else {
                resultado.textBlocks.asSequence()
                    .flatMap { it.lines }
                    .filter { esLineaMrzProbable(it.text) }
                    .mapNotNull { linea ->
                        linea.boundingBox?.let {
                            FraccionesRect.desdePixeles(it.left, it.top, it.right, it.bottom, recorte.anchoVertical, recorte.altoVertical)
                        }
                    }
                    .toList()
            }
            onLectura(
                LecturaFrame(LectorFrame.TEXTO, resultado.text, null, calidad, decision.peso, regionLeida, lineasMrz),
            )
        }.cerrarAlTerminar(imagen, camara, onFallo)
    }
}

/// Fallo y cierre comunes a los dos lectores, en el hilo del analizador.
/// Cerrar el `ImageProxy` al COMPLETAR (no antes) es lo que hace que
/// CameraX no entregue otro frame mientras éste se procesa.
private fun <T> com.google.android.gms.tasks.Task<T>.cerrarAlTerminar(
    imagen: ImageProxy,
    camara: EstadoCamaraOcr,
    onFallo: () -> Unit,
) {
    addOnFailureListener(camara.ejecutorProcesamiento) {
        camara.metricas.registrarFallo()
        if (camara.sesionActiva.get()) onFallo()
    }.addOnCompleteListener(camara.ejecutorProcesamiento) { imagen.close() }
}

/// Recorte listo para ML Kit, más lo necesario para medir su calidad
/// (luminancia en orientación del sensor) y para ubicar las cajas que
/// devuelve ML Kit (que llegan ya rotadas, en `anchoVertical` x
/// `altoVertical`).
private class RecorteOcr(
    val input: InputImage,
    val nv21: ByteArray,
    val anchoSensor: Int,
    val altoSensor: Int,
    val anchoVertical: Int,
    val altoVertical: Int,
)

/// Recorta el frame de la cámara a `region` (el recuadro que dibuja
/// `MarcoGuiaCedula`, o una parte de él) antes de mandarlo a ML Kit.
/// `null` si algo no sale como se espera -- el llamador cae de vuelta al
/// frame completo, nunca debe romper el escaneo.
///
/// Recorta DIRECTO sobre los planos YUV del sensor: [rectanguloEnSensor]
/// traduce la región (coordenadas de pantalla, ya rotadas) al espacio sin
/// rotar del sensor -- con tests para las 4 rotaciones -- y
/// [recortarYuvANv21] copia sólo esa región a NV21, leyendo fila por fila
/// de cada `ByteBuffer` (nunca los planos enteros). ML Kit recibe el NV21
/// con la rotación y la aplica él.
///
/// `cropRect` (MV-08, auditoría 2026-09-24): el rectángulo que CameraX
/// calculó a partir del `ViewPort`, en coordenadas del búfer SIN rotar --
/// sin él, el recuadro guía se calculaba sobre el frame entero del sensor,
/// que casi nunca tiene la misma proporción que lo que se ve en pantalla
/// con `FILL_CENTER`.
@androidx.annotation.OptIn(ExperimentalGetImage::class)
private fun recortarParaOcr(
    imagen: android.media.Image,
    cropRect: android.graphics.Rect,
    rotacionGrados: Int,
    region: RegionRecorte,
): RecorteOcr? {
    if (imagen.format != android.graphics.ImageFormat.YUV_420_888) return null
    val planos = imagen.planes
    if (planos.size < 3) return null
    return try {
        val yPlano = planos[0]
        val uPlano = planos[1]
        val vPlano = planos[2]
        // U y V comparten geometría en YUV_420_888 salvo en implementaciones
        // de cámara anómalas; si no, mejor el frame completo que un croma
        // leído con los pasos equivocados.
        if (uPlano.rowStride != vPlano.rowStride || uPlano.pixelStride != vPlano.pixelStride) return null

        val cropSeguro = android.graphics.Rect(cropRect)
        if (!cropSeguro.intersect(0, 0, imagen.width, imagen.height)) {
            cropSeguro.set(0, 0, imagen.width, imagen.height)
        }
        val recorte = rectanguloEnSensor(
            RectanguloEntero(cropSeguro.left, cropSeguro.top, cropSeguro.right, cropSeguro.bottom),
            rotacionGrados,
            region,
        )
        if (recorte.width <= 0 || recorte.height <= 0 ||
            recorte.right > imagen.width || recorte.bottom > imagen.height
        ) {
            return null
        }
        val nv21 = recortarYuvANv21(
            recorte,
            y = yPlano.buffer,
            yRowStride = yPlano.rowStride,
            u = uPlano.buffer,
            v = vPlano.buffer,
            uvRowStride = uPlano.rowStride,
            uvPixelStride = uPlano.pixelStride,
        )
        val rotada = rotacionGrados % 180 != 0
        RecorteOcr(
            input = InputImage.fromByteArray(nv21, recorte.width, recorte.height, rotacionGrados, InputImage.IMAGE_FORMAT_NV21),
            nv21 = nv21,
            anchoSensor = recorte.width,
            altoSensor = recorte.height,
            anchoVertical = if (rotada) recorte.height else recorte.width,
            altoVertical = if (rotada) recorte.width else recorte.height,
        )
    } catch (e: Exception) {
        null
    }
}
