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
const val MENSAJE_FALLO_LECTURA_OCR = "No se pudo leer. Acerque el documento y evite reflejos."

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
    // función (copiar los planos y recortar a NV21), no
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
/// Recorta DIRECTO sobre los planos YUV del sensor: [rectanguloEnSensor]
/// traduce el recuadro (coordenadas de pantalla, ya rotadas) al espacio sin
/// rotar del sensor -- con tests para las 4 rotaciones, que es justo el
/// mapeo que antes se evitaba rotando un bitmap completo primero -- y
/// [recortarYuvANv21] copia sólo esa región a NV21. ML Kit recibe el NV21 con
/// la rotación y la aplica él. Antes: frame entero a ARGB en un bucle de
/// Kotlin, bitmap de ~4 MB, recorte al viewport, rotación del bitmap
/// entero y otro recorte -- el costo de CPU más alto por frame, y además
/// con una conversión de color que saturaba los reflejos (ver
/// [recortarYuvANv21]).
///
/// `cropRect` (MV-08, auditoría 2026-09-24): el rectángulo que CameraX
/// calculó a partir del `ViewPort`, en coordenadas del búfer SIN rotar --
/// sin él, el recuadro guía se calculaba sobre el frame entero del sensor,
/// que casi nunca tiene la misma proporción que lo que se ve en pantalla
/// con `FILL_CENTER` (podía leer un documento fuera del marco).
///
/// Los planos se leen con `duplicate()`: no mueve la posición de los
/// `ByteBuffer` de la imagen, así que si algo falla después, el fallback
/// `InputImage.fromMediaImage` todavía encuentra los planos intactos (antes
/// quedaban ya consumidos).
@androidx.annotation.OptIn(ExperimentalGetImage::class)
private fun recortarParaOcr(
    imagen: android.media.Image,
    cropRect: android.graphics.Rect,
    rotacionGrados: Int,
    region: RegionGuiaOcr,
    buffersOcr: BuffersOcrReutilizables? = null,
): InputImage? {
    if (imagen.format != android.graphics.ImageFormat.YUV_420_888) return null
    val planos = imagen.planes
    if (planos.size < 3) return null
    return try {
        val yPlano = planos[0]
        val uPlano = planos[1]
        val vPlano = planos[2]
        val yBuffer = yPlano.buffer.duplicate()
        val uBuffer = uPlano.buffer.duplicate()
        val vBuffer = vPlano.buffer.duplicate()
        val yBytes = (buffersOcr?.yBytes(yBuffer.remaining()) ?: ByteArray(yBuffer.remaining())).also { yBuffer.get(it) }
        val uBytes = (buffersOcr?.uBytes(uBuffer.remaining()) ?: ByteArray(uBuffer.remaining())).also { uBuffer.get(it) }
        val vBytes = (buffersOcr?.vBytes(vBuffer.remaining()) ?: ByteArray(vBuffer.remaining())).also { vBuffer.get(it) }

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
            y = yBytes,
            yRowStride = yPlano.rowStride,
            u = uBytes,
            v = vBytes,
            uvRowStride = uPlano.rowStride,
            uvPixelStride = uPlano.pixelStride,
        )
        InputImage.fromByteArray(nv21, recorte.width, recorte.height, rotacionGrados, InputImage.IMAGE_FORMAT_NV21)
    } catch (e: Exception) {
        null
    }
}
