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
import com.google.android.gms.tasks.Tasks
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.Text
import uniffi.control_acceso_mobile.DatosPdf417Cedula
import uniffi.control_acceso_mobile.LineaOcr
import uniffi.control_acceso_mobile.MotivoPdf417
import uniffi.control_acceso_mobile.PalabraOcr
import uniffi.control_acceso_mobile.confianzaMinimaPalabra
import uniffi.control_acceso_mobile.textosDeFrame
import java.util.concurrent.atomic.AtomicBoolean

/// Plomería de cámara + ML Kit compartida por las pantallas de escaneo
/// (sacada de `PantallaEscanearCedula.kt`, punto M4 de la auditoría
/// móvil): analizador de CameraX con límite de frames, arranque de la
/// cámara, recorte al recuadro guía sobre los planos YUV y entrega a ML
/// Kit. Sin nada de clasificación de documentos -- eso vive en el núcleo
/// Rust (`mobile/rust-core/src/lectura_documentos/`).

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
    // ¿Hay lugar para otro frame en proceso? (ver
    // `EstadoCamaraOcr.hayLugar`). Si no, el frame se descarta ANTES de
    // recortar: nunca se forma una cola atrasada.
    hayLugar: () -> Boolean = { true },
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
                if (ahora - ultimoFrameProcesadoMs < INTERVALO_MINIMO_ENTRE_FRAMES_MS || !hayLugar()) {
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

// Techo de frames que entran a la tubería (~12/s). Antes era 150 ms (~6-7/s)
// con UN frame a la vez; ahora el límite real lo pone la cantidad de frames
// en proceso (`MAXIMO_EN_PROCESO`, en `EstadoCamaraOcr.kt`) y el filtro de calidad,
// y esto sólo evita recortar y medir frames que igual no tendrían lugar.
// El sensor sigue entregando a su fps normal. Ajustable con las métricas
// (`adb logcat -s OcrMetricas`: fps y mediana de reconocimiento).
private const val INTERVALO_MINIMO_ENTRE_FRAMES_MS = 80L

/// Compartido por las 4 pantallas de escaneo -- antes cada una tenía su
/// propia copia textual idéntica (hallazgo 2026-09-19, riesgo de
/// desincronizarse si alguien edita una sin las otras 3). Sin `private`:
/// vive acá porque [analizarFrameOcr]/[iniciarCamara] (el resto de lo
/// compartido) también viven en este archivo.
const val MENSAJE_FALLO_LECTURA_OCR = "No se pudo leer. Acerque el documento y evite reflejos."

/// Analiza un frame: recorta a `region`, mide su calidad y corre ML Kit
/// -- texto siempre y, si `leerCodigo`, el PDF417 de la cédula anterior EN
/// PARALELO sobre el mismo recorte. Entrega una [LecturaFrame]. No sabe
/// nada de cédulas ni de placas: clasificar y decidir vive en los
/// estabilizadores.
///
/// Paralelismo (auditoría OCR 2026-09-28):
/// - El `ImageProxy` se cierra APENAS se copia el recorte (el NV21 es una
///   copia propia): la cámara prepara el frame siguiente mientras ML Kit
///   procesa éste. Sólo si no se pudo recortar se retiene hasta el final
///   (ML Kit lee entonces el frame original).
/// - Hasta `MAXIMO_EN_PROCESO` frames a la vez; el lugar
///   se reserva acá y se libera cuando terminan TODOS sus lectores.
/// - Texto y código corren a la vez (dos modelos de ML Kit): buscar el
///   PDF417 no le quita frames al texto.
///
/// `onLectura`/`onFallo` corren en el HILO DEL ANALIZADOR, uno por frame y
/// nunca dos a la vez (es un solo hilo): clasificar, votar y cruzar a Rust
/// no traba la pantalla y queda serializado. Con dos frames en vuelo
/// pueden terminar en otro orden; la votación no depende del orden. Quien
/// llama publica al hilo principal sólo el estado de pantalla (ver
/// [EstadoCamaraOcr.enPrincipal]).
@androidx.annotation.OptIn(ExperimentalGetImage::class)
fun analizarFrameOcr(
    imagen: ImageProxy,
    camara: EstadoCamaraOcr,
    region: RegionRecorte?,
    leerCodigo: Boolean = false,
    // "Modo código" (ver `PlanificadorLectores`): el lector de códigos recibe
    // un segundo recorte con TODO lo visible en vez del recuadro guía. El
    // texto sigue con su recorte de siempre. Sólo cuenta si `leerCodigo`.
    modoCodigo: Boolean = false,
    onLectura: (LecturaFrame) -> Unit,
    onFallo: () -> Unit,
) {
    val mediaImage = imagen.image
    // Se repite el chequeo de sesión justo antes del trabajo pesado: si la
    // pantalla se cerró entre medio, no se gasta CPU en un frame que nadie
    // va a usar.
    if (mediaImage == null || !camara.sesionActiva.get() || !camara.reservarLugar()) {
        imagen.close()
        return
    }
    // Quién debe cerrar la imagen y liberar el lugar: esta función hasta
    // que ML Kit recibe el frame; después, el listener.
    var pendienteDeCierre = true
    try {
        val inicio = System.nanoTime()
        val rotacion = imagen.imageInfo.rotationDegrees
        // MV-08 (auditoría 2026-09-24): `cropRect` es lo que CameraX ajusta
        // a partir del `ViewPort` -- la porción del búfer que de verdad se
        // ve en pantalla.
        val recorte = region?.let { recortarParaOcr(mediaImage, imagen.cropRect, rotacion, it) }
        val calidad = recorte?.let { medirCalidad(it.nv21, it.anchoSensor, it.altoSensor) }
        // Sin región no hay recorte ni medición: se procesa con peso 1.
        calidad?.let { camara.metricas.registrarCalidad(it, camara.linternaEncendida) }
        val decision = if (region != null && calidad != null) camara.filtroCalidad(region).evaluar(calidad) else SIN_MEDICION
        if (!decision.procesar) {
            camara.metricas.registrarDescarte()
            return
        }
        val lectorCodigos = camara.lectorCodigos
        // Segundo recorte, sólo para el lector de códigos: es una copia
        // propia (NV21), así que no retiene el `ImageProxy`. Si no se puede
        // armar, el código se lee del recorte normal.
        val recorteCodigo = if (leerCodigo && modoCodigo && lectorCodigos != null) {
            recortarParaOcr(mediaImage, imagen.cropRect, rotacion, TodoLoVisibleRecorte)
        } else {
            null
        }
        val finRecorte = System.nanoTime()
        // Antes de cerrar la imagen: el ancho que ve ML Kit (ya rotado).
        val anchoImagenPx = recorte?.anchoVertical ?: if (rotacion % 180 != 0) mediaImage.height else mediaImage.width
        // Sin recorte (formato inesperado, plano raro) se lee el frame
        // entero: un recorte fallido nunca debe romper el escaneo.
        val input = recorte?.input ?: InputImage.fromMediaImage(mediaImage, rotacion)
        val retenerImagen = recorte == null
        if (!retenerImagen) imagen.close()
        val regionLeida = if (recorte != null) region else null
        val tareaTexto = camara.recognizer.process(input)
        val tareaCodigo = if (leerCodigo && lectorCodigos != null) lectorCodigos.process(recorteCodigo?.input ?: input) else null
        val anchoCodigoPx = recorteCodigo?.anchoVertical ?: anchoImagenPx
        val leyoEnModoCodigo = recorteCodigo != null
        pendienteDeCierre = false
        Tasks.whenAllComplete(listOfNotNull(tareaTexto, tareaCodigo)).addOnCompleteListener(camara.ejecutorProcesamiento) {
            try {
                if (!camara.sesionActiva.get()) return@addOnCompleteListener
                camara.metricas.registrarFrame(tareaCodigo != null, finRecorte - inicio, System.nanoTime() - finRecorte, leyoEnModoCodigo)
                val texto = tareaTexto.takeIf { it.isSuccessful }?.result
                if (tareaCodigo != null && !tareaCodigo.isSuccessful) camara.metricas.registrarErrorLectorCodigo()
                val datosPdf417 = tareaCodigo?.takeIf { it.isSuccessful }?.result
                    ?.let { codigos -> leerCodigos(codigos, anchoCodigoPx, leyoEnModoCodigo, camara.metricas) }
                if (texto == null && datosPdf417 == null) {
                    camara.metricas.registrarFallo()
                    onFallo()
                    return@addOnCompleteListener
                }
                val lineasMrz = if (recorte == null || texto == null) {
                    emptyList()
                } else {
                    texto.textBlocks.asSequence()
                        .flatMap { it.lines }
                        .filter { esLineaMrzProbable(it.text) }
                        .mapNotNull { linea ->
                            linea.boundingBox?.let {
                                FraccionesRect.desdePixeles(it.left, it.top, it.right, it.bottom, recorte.anchoVertical, recorte.altoVertical)
                            }
                        }
                        .toList()
                }
                val textos = texto?.let { textosParaLectores(it, camara.metricas) }.orEmpty()
                onLectura(LecturaFrame(texto?.text.orEmpty(), textos, datosPdf417, calidad, decision.peso, regionLeida, lineasMrz))
            } finally {
                if (retenerImagen) imagen.close()
                camara.liberarLugar()
            }
        }
    } catch (e: RuntimeException) {
        // Falla antes de entregar el frame a ML Kit: se suelta todo abajo.
        camara.metricas.registrarFallo()
    } finally {
        if (pendienteDeCierre) {
            imagen.close()
            camara.liberarLugar()
        }
    }
}

/// Las versiones del texto del frame para los lectores (auditoría OCR
/// 2026-09-28, E-3): el núcleo arma los renglones visuales con las cajas de
/// cada línea y palabra (y su confianza) y agrega el texto original de ML
/// Kit como segunda opción. Las cajas van en píxeles de la imagen que
/// analizó ML Kit; sólo importan sus posiciones relativas. Una línea o
/// palabra sin caja se omite de la versión visual (la original la conserva).
private fun textosParaLectores(texto: Text, metricas: MetricasOcr): List<String> {
    val lineas = texto.textBlocks.flatMap { bloque ->
        bloque.lines.mapNotNull { linea ->
            val caja = linea.boundingBox ?: return@mapNotNull null
            LineaOcr(
                texto = linea.text,
                izquierda = caja.left.toFloat(),
                arriba = caja.top.toFloat(),
                derecha = caja.right.toFloat(),
                abajo = caja.bottom.toFloat(),
                palabras = linea.elements.mapNotNull { palabra ->
                    val c = palabra.boundingBox ?: return@mapNotNull null
                    PalabraOcr(
                        texto = palabra.text,
                        izquierda = c.left.toFloat(),
                        arriba = c.top.toFloat(),
                        derecha = c.right.toFloat(),
                        abajo = c.bottom.toFloat(),
                        confianza = palabra.confidence,
                    )
                },
            )
        }
    }
    val textos = textosDeFrame(texto.text, lineas)
    metricas.registrarTexto(lineas.flatMap { l -> l.palabras.map { it.confianza } }, UMBRAL_CONFIANZA_PALABRA, textos.size)
    return textos
}

/// El mismo umbral con que el núcleo descarta palabras (ver
/// `renglones_visuales.rs`), para contarlas en la telemetría.
private val UMBRAL_CONFIANZA_PALABRA = confianzaMinimaPalabra()

/// El primer PDF417 de la cédula anterior que el núcleo acepta, y el
/// diagnóstico de todos los códigos del frame para la telemetría. Los
/// bytes crudos se ponen en cero en todos los casos (ver
/// [leerPdf417ConMotivo]).
private fun leerCodigos(
    codigos: List<Barcode>,
    anchoImagenPx: Int,
    enModoCodigo: Boolean,
    metricas: MetricasOcr,
): DatosPdf417Cedula? {
    var datos: DatosPdf417Cedula? = null
    var sinBytes = 0
    val motivos = mutableListOf<MotivoPdf417>()
    val largos = mutableListOf<Int>()
    val anchos = mutableListOf<Int>()
    for (codigo in codigos) {
        codigo.boundingBox?.let { anchos += it.width() }
        val crudo = codigo.rawBytes
        if (crudo == null) {
            sinBytes++
            continue
        }
        largos += crudo.size
        if (datos != null) {
            crudo.fill(0)
            continue
        }
        val lectura = leerPdf417ConMotivo(crudo)
        motivos += lectura.motivo
        datos = lectura.datos
    }
    metricas.registrarBusquedaCodigo(DiagnosticoCodigos(codigos.size, sinBytes, motivos, largos, anchos, anchoImagenPx, enModoCodigo))
    return datos
}

/// Frame sin recorte (y por lo tanto sin medición): se procesa con peso 1.
private val SIN_MEDICION = DecisionCalidad(procesar = true, peso = 1f)

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
