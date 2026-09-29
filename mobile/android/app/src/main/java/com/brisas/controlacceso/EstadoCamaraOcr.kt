package com.brisas.controlacceso

import android.content.Context
import android.util.Log
import androidx.camera.core.Camera
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.camera.core.UseCaseGroup
import androidx.camera.core.ViewPort
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.core.content.ContextCompat
import androidx.lifecycle.LifecycleOwner
import com.google.mlkit.vision.barcode.BarcodeScanner
import com.google.mlkit.vision.barcode.BarcodeScannerOptions
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.TextRecognizer
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import java.util.concurrent.Executor
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.Job

/// Recursos de una sesión de escaneo, compartidos por las 4 pantallas
/// (Cédula/Gafete, Carnet KOF, Vehículo/Ruta, Comprobante de Ruta) -- MV-10
/// (auditoría 2026-09-24): antes cada pantalla los declaraba y liberaba por
/// separado.
///
/// `conLectorPdf417`: sólo la pantalla de documentos lo pide (cédula
/// anterior); el resto no carga el modelo de códigos.
///
/// `mutableStateOf` (no propiedades simples) para lo que se asigna DESPUÉS
/// de construir el estado, dentro del `factory` de `AndroidView`, y para lo
/// que la pantalla dibuja (linterna).
class EstadoCamaraOcr(contexto: Context, conLectorPdf417: Boolean = false) {
    val ejecutor: ExecutorService = Executors.newSingleThreadExecutor()
    val recognizer: TextRecognizer = TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS)

    /// Sólo PDF417: pedir un formato hace el escaneo más rápido que buscar
    /// los 13 que soporta ML Kit.
    val lectorCodigos: BarcodeScanner? = if (conLectorPdf417) {
        BarcodeScanning.getClient(BarcodeScannerOptions.Builder().setBarcodeFormats(Barcode.FORMAT_PDF417).build())
    } else {
        null
    }
    val ejecutorPrincipal: Executor = ContextCompat.getMainExecutor(contexto)

    /// El hilo del analizador, para los listeners de ML Kit (ver
    /// `analizarFrameOcr`). Si ya se liberó, corre el listener en el lugar:
    /// éste ve `sesionActiva == false`, no procesa nada, y el `ImageProxy`
    /// igual se cierra.
    val ejecutorProcesamiento: Executor = Executor { tarea ->
        try {
            ejecutor.execute(tarea)
        } catch (_: RejectedExecutionException) {
            tarea.run()
        }
    }
    val detectada = AtomicBoolean(false)
    val sesionActiva = AtomicBoolean(true)

    // Frames que están ahora en ML Kit. Se reservan y liberan en el hilo
    // del analizador (analizador y listeners comparten ese hilo), pero es
    // atómico por si algún día no fuera así.
    private val enProceso = AtomicInteger(0)

    /// ¿Entra otro frame? Lo consulta el analizador antes de recortar.
    fun hayLugar(): Boolean = enProceso.get() < MAXIMO_EN_PROCESO

    /// Reserva un lugar para un frame; `false` si ya hay
    /// [MAXIMO_EN_PROCESO] en vuelo.
    fun reservarLugar(): Boolean {
        while (true) {
            val actual = enProceso.get()
            if (actual >= MAXIMO_EN_PROCESO) return false
            if (enProceso.compareAndSet(actual, actual + 1)) return true
        }
    }

    fun liberarLugar() {
        enProceso.updateAndGet { (it - 1).coerceAtLeast(0) }
    }

    // Un filtro por tipo de región: la nitidez sólo es comparable entre
    // recortes del mismo contenido (la banda del MRZ, texto denso, "parece"
    // más nítida que la tarjeta entera). Sólo lo usa el hilo del analizador.
    private val filtrosCalidad = HashMap<Pair<RegionRecorte, Boolean>, FiltroCalidad>()

    fun filtroCalidad(region: RegionRecorte): FiltroCalidad {
        val clave = when (region) {
            is SubregionRecorte -> region.base to true
            else -> region to false
        }
        return filtrosCalidad.getOrPut(clave) { FiltroCalidad() }
    }

    /// Números de rendimiento por sesión (sin datos personales): en debug
    /// por `adb logcat -s OcrMetricas`; en el build `diagnostico` se mandan
    /// a la telemetría al liberar la cámara.
    val metricas = MetricasOcr(
        habilitadas = BuildConfig.DEBUG || Telemetria.activa,
        registrar = { if (BuildConfig.DEBUG) Log.d(TAG_METRICAS_OCR, it) },
    )

    var cameraProvider: ProcessCameraProvider? by mutableStateOf(null)
    var vistaPreviaCamara: Preview? by mutableStateOf(null)
    var analisisCamara: ImageAnalysis? by mutableStateOf(null)

    /// Mismo analizador a la mayor resolución: sólo mientras dura el "modo
    /// código" del PDF417 (ver [usarAnalisisDeCodigo]). `null` = la pantalla
    /// no lo usa.
    var analisisCodigo: ImageAnalysis? = null

    /// Lo que hace falta para volver a enlazar (ver `iniciarCamara`).
    private var propietarioCamara: LifecycleOwner? = null
    private var viewPortCamara: ViewPort? = null

    /// ¿Está enlazado [analisisCodigo] en vez de [analisisCamara]?
    private var analisisCodigoActivo = false
    var trabajoResultado: Job? by mutableStateOf(null)
    var camaraFisica: Camera? by mutableStateOf(null)
    var linternaEncendida by mutableStateOf(false)
        private set

    val tieneLinterna: Boolean get() = camaraFisica?.cameraInfo?.hasFlashUnit() == true

    init {
        precalentar()
    }

    /// De noche en la portería, o con placas en sombra, la luz es lo que
    /// más limita la lectura.
    fun alternarLinterna() {
        val camara = camaraFisica ?: return
        if (!tieneLinterna) return
        val encender = !linternaEncendida
        camara.cameraControl.enableTorch(encender)
        linternaEncendida = encender
    }

    /// Lo llama `iniciarCamara` cuando la cámara quedó enlazada.
    fun alEnlazar(propietario: LifecycleOwner, viewPort: ViewPort?) {
        propietarioCamara = propietario
        viewPortCamara = viewPort
    }

    /// Cambia el análisis por el de alta resolución (`activar`) o vuelve al
    /// normal, sin soltar el preview. En el hilo principal (lo exige
    /// CameraX). La cámara se reconfigura: el preview se congela un momento
    /// al entrar y al salir del modo código. Si el equipo no admite la
    /// combinación, se queda (o vuelve) con el normal: el escaneo sigue
    /// como siempre.
    fun usarAnalisisDeCodigo(activar: Boolean) {
        if (!sesionActiva.get() || activar == analisisCodigoActivo) return
        val proveedor = cameraProvider ?: return
        val preview = vistaPreviaCamara ?: return
        val propietario = propietarioCamara ?: return
        val normal = analisisCamara ?: return
        val alto = analisisCodigo ?: return
        val (sale, entra) = if (activar) normal to alto else alto to normal
        val inicio = android.os.SystemClock.elapsedRealtime()
        try {
            enlazarConPreview(proveedor, propietario, preview, sale, entra)
            analisisCodigoActivo = activar
            metricas.registrarCambioAnalisisCodigo(activar, exito = true, ms = android.os.SystemClock.elapsedRealtime() - inicio)
            if (activar) {
                camaraFisica?.let { reportarCamaraInfo(it, alto, preview, propietario, mapOf("modo_codigo" to true)) }
            }
        } catch (_: RuntimeException) {
            metricas.registrarCambioAnalisisCodigo(activar, exito = false, ms = android.os.SystemClock.elapsedRealtime() - inicio)
            analisisCodigoActivo = false
            if (activar) {
                // Volver al de siempre; si tampoco se puede, la pantalla ya
                // mostraba el error de cámara al arrancar.
                try {
                    enlazarConPreview(proveedor, propietario, preview, alto, normal)
                } catch (_: RuntimeException) {
                }
            }
        }
    }

    private fun enlazarConPreview(
        proveedor: ProcessCameraProvider,
        propietario: LifecycleOwner,
        preview: Preview,
        sale: ImageAnalysis,
        entra: ImageAnalysis,
    ) {
        proveedor.unbind(sale)
        val grupo = UseCaseGroup.Builder()
            .addUseCase(preview)
            .addUseCase(entra)
            .apply { viewPortCamara?.let { setViewPort(it) } }
            .build()
        val camara = proveedor.bindToLifecycle(propietario, CameraSelector.DEFAULT_BACK_CAMERA, grupo)
        camaraFisica = camara
        // Reenlazar puede apagar la linterna: se respeta lo que eligió
        // quien opera.
        if (linternaEncendida) camara.cameraControl.enableTorch(true)
    }

    /// Ejecuta `bloque` en el hilo principal si la sesión sigue viva. Los
    /// resultados se calculan en el hilo del analizador; sólo el estado de
    /// pantalla y los efectos (vibración, sonido) se publican acá.
    fun enPrincipal(bloque: () -> Unit) {
        ejecutorPrincipal.execute { if (sesionActiva.get()) bloque() }
    }

    /// El primer `process` carga el modelo (cientos de ms). Hacerlo con
    /// una imagen mínima mientras la cámara arranca -- que igual tarda --
    /// evita que ese costo caiga sobre el primer frame real.
    private fun precalentar() {
        val vacia = InputImage.fromByteArray(ByteArray(32 * 32 * 3 / 2), 32, 32, 0, InputImage.IMAGE_FORMAT_NV21)
        recognizer.process(vacia)
        lectorCodigos?.process(vacia)
    }

    /// Mismo cierre que cada pantalla hacía a mano en su propio
    /// `onDispose` -- invalida callbacks de CameraX/ML Kit que terminen
    /// después de salir de la composición, desata la cámara del ciclo de
    /// vida anterior, y libera hilo y modelos.
    fun liberar() {
        sesionActiva.set(false)
        detectada.set(true)
        trabajoResultado?.cancel()
        analisisCamara?.clearAnalyzer()
        analisisCodigo?.clearAnalyzer()
        val casos = listOfNotNull(vistaPreviaCamara, analisisCamara, analisisCodigo).toTypedArray()
        if (casos.isNotEmpty()) cameraProvider?.unbind(*casos)
        ejecutor.shutdown()
        recognizer.close()
        lectorCodigos?.close()
        if (Telemetria.activa) {
            Telemetria.evento(
                "ocr_sesion",
                metricas.datos() + mapOf(
                    "pantalla" to Telemetria.pantallaActual(),
                    "abierta_desde" to Telemetria.pantallaAnterior(),
                    "lector_pdf417" to (lectorCodigos != null),
                ),
            )
            // La cámara y sus modelos retienen búferes grandes: si este
            // estado sigue vivo después de cerrar la pantalla, hay una fuga.
            Telemetria.vigilarRetencion(this, "EstadoCamaraOcr")
        }
    }
}

private const val TAG_METRICAS_OCR = "OcrMetricas"

/// Frames en ML Kit a la vez. Con 2, mientras uno se reconoce el siguiente
/// ya se recorta y mide (y la cámara prepara el que sigue): el hilo del
/// analizador y los núcleos libres dejan de esperar. Más de 2 sólo
/// sumaría latencia por frame (ML Kit se reparte los núcleos) y calor.
const val MAXIMO_EN_PROCESO = 2

/// Crea un [EstadoCamaraOcr] atado al ciclo de vida de esta composición --
/// `liberar()` corre una sola vez, al salir.
@Composable
fun rememberEstadoCamaraOcr(contexto: Context, conLectorPdf417: Boolean = false): EstadoCamaraOcr {
    val estado = remember { EstadoCamaraOcr(contexto, conLectorPdf417) }
    DisposableEffect(Unit) {
        estado.sesionActiva.set(true)
        onDispose { estado.liberar() }
    }
    return estado
}
