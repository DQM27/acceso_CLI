package com.brisas.controlacceso

import android.content.Context
import android.util.Log
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.core.content.ContextCompat
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
/// `mutableStateOf` (no propiedades simples) para lo que se asigna DESPUÉS
/// de construir el estado, dentro del `factory` de `AndroidView`.
/// El lector de texto de ML Kit, uno solo para todo el proceso (2026-10-27).
/// Antes cada apertura de cámara creaba el suyo, cargaba el modelo (cientos
/// de ms de CPU, justo mientras se abría la cámara) y lo cerraba al salir:
/// cada escaneo pagaba esa carga de nuevo. Ahora se carga una vez y queda en
/// memoria. Nunca se cierra: vive lo que vive el proceso.
///
/// El cliente lee del disco al crearse: StrictMode lo marcaba en el hilo
/// principal (staging, 2026-10-19). Se crea en el primer hilo que lo pide,
/// que nunca es el principal: [precalentar] corre en segundo plano y las
/// cámaras lo usan desde el hilo del analizador.
internal object LectorTextoOcr {
    val cliente: TextRecognizer by lazy { TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS) }
    private val precalentado = AtomicBoolean(false)

    /// El primer `process` carga el modelo. Con una imagen mínima, una sola
    /// vez por proceso; las llamadas siguientes no hacen nada.
    fun precalentar() {
        if (!precalentado.compareAndSet(false, true)) return
        val vacia = InputImage.fromByteArray(ByteArray(32 * 32 * 3 / 2), 32, 32, 0, InputImage.IMAGE_FORMAT_NV21)
        cliente.process(vacia)
    }
}

class EstadoCamaraOcr(contexto: Context) {
    val ejecutor: ExecutorService = Executors.newSingleThreadExecutor()

    /// El de todo el proceso ([LectorTextoOcr]).
    val recognizer: TextRecognizer get() = LectorTextoOcr.cliente
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
    /// por `adb logcat -s OcrMetricas`; con el grupo `escaneo` se mandan a la
    /// telemetría al liberar la cámara.
    val metricas = MetricasOcr(
        habilitadas = BuildConfig.DEBUG || Telemetria.permite("ocr_sesion"),
        registrar = { if (BuildConfig.DEBUG) Log.d(TAG_METRICAS_OCR, it) },
    )

    var cameraProvider: ProcessCameraProvider? by mutableStateOf(null)
    var vistaPreviaCamara: Preview? by mutableStateOf(null)
    var analisisCamara: ImageAnalysis? by mutableStateOf(null)
    var trabajoResultado: Job? by mutableStateOf(null)

    /// `true` desde que la vista previa muestra imagen. Hasta ahí la pantalla
    /// queda en negro, sin el recuadro: abrir la cámara física tarda unos
    /// 0,75 s (telemetría de staging, `ms_hasta_imagen_camara`, igual desde
    /// la 1.6.3) y el recuadro encima del negro hacía notar esa espera como
    /// una pantalla de carga (2026-10-27).
    var vistaPreviaLista by mutableStateOf(false)
        private set

    /// La llama [iniciarCamara] con la primera imagen de la vista previa.
    fun marcarVistaPreviaLista() {
        metricas.registrarVistaPrevia()
        vistaPreviaLista = true
    }

    init {
        // Normalmente ya lo precalentó la Activity al arrancar; si no, se hace
        // acá, en el hilo del analizador.
        ejecutor.execute(LectorTextoOcr::precalentar)
    }

    /// Ejecuta `bloque` en el hilo principal si la sesión sigue viva. Los
    /// resultados se calculan en el hilo del analizador; sólo el estado de
    /// pantalla y los efectos (vibración, sonido) se publican acá.
    fun enPrincipal(bloque: () -> Unit) {
        ejecutorPrincipal.execute { if (sesionActiva.get()) bloque() }
    }

    /// Mismo cierre que cada pantalla hacía a mano en su propio
    /// `onDispose` -- invalida callbacks de CameraX/ML Kit que terminen
    /// después de salir de la composición, desata la cámara del ciclo de
    /// vida anterior, y libera el hilo. El modelo de ML Kit no se cierra: es
    /// el de todo el proceso ([LectorTextoOcr]).
    fun liberar() {
        val inicioCierre = System.nanoTime()
        sesionActiva.set(false)
        detectada.set(true)
        trabajoResultado?.cancel()
        analisisCamara?.clearAnalyzer()
        val casos = listOfNotNull(vistaPreviaCamara, analisisCamara).toTypedArray()
        if (casos.isNotEmpty()) cameraProvider?.unbind(*casos)
        val finUnbind = System.nanoTime()
        // El hilo termina lo que quede en su cola (los frames ya ven la sesión
        // cerrada y no procesan nada) y se apaga.
        if (!ejecutor.isShutdown) ejecutor.shutdown()
        val finCierre = System.nanoTime()
        if (Telemetria.permite("ocr_sesion")) {
            Telemetria.evento(
                "ocr_sesion",
                metricas.datos() + mapOf(
                    "pantalla" to Telemetria.pantallaActual(),
                    "abierta_desde" to Telemetria.pantallaAnterior(),
                    // Cuánto tardó cerrar la cámara en el hilo que la cierra (el
                    // principal): `unbind` y luego apagar el hilo del analizador
                    // (`ms_cierre_modelos` conserva el nombre de cuando ahí se
                    // cerraba también el modelo de ML Kit, para poder comparar
                    // con versiones anteriores). Si esto es grande, la pantalla
                    // se traba justo al salir del escaneo.
                    "ms_cierre_unbind" to milisegundosDesde(inicioCierre, finUnbind),
                    "ms_cierre_modelos" to milisegundosDesde(finUnbind, finCierre),
                    "ms_cierre_total" to milisegundosDesde(inicioCierre, finCierre),
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
fun rememberEstadoCamaraOcr(contexto: Context): EstadoCamaraOcr {
    val estado = remember { EstadoCamaraOcr(contexto) }
    DisposableEffect(Unit) {
        estado.sesionActiva.set(true)
        onDispose { estado.liberar() }
    }
    return estado
}
