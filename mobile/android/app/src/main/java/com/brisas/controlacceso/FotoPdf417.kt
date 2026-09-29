package com.brisas.controlacceso

import android.os.SystemClock
import android.util.Size
import androidx.camera.core.ImageCapture
import androidx.camera.core.ImageCaptureException
import androidx.camera.core.ImageProxy
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import com.google.mlkit.vision.barcode.BarcodeScanner
import com.google.mlkit.vision.common.InputImage
import uniffi.control_acceso_mobile.DatosPdf417Cedula
import java.util.concurrent.Executor
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.atomic.AtomicBoolean

/// Caso de uso de foto para el PDF417: la mayor resolución del sensor
/// hasta ~12 MP. El tope importa: en un sensor de 50 MP la foto sin tope
/// ocuparía ~200 MB como imagen en memoria.
fun construirCapturaPdf417(): ImageCapture = ImageCapture.Builder()
    .setCaptureMode(ImageCapture.CAPTURE_MODE_MINIMIZE_LATENCY)
    .setFlashMode(ImageCapture.FLASH_MODE_OFF)
    .setResolutionSelector(
        ResolutionSelector.Builder()
            .setResolutionStrategy(
                ResolutionStrategy(Size(4032, 3024), ResolutionStrategy.FALLBACK_RULE_CLOSEST_LOWER_THEN_HIGHER),
            )
            .build(),
    )
    .build()

/// Lee el PDF417 del reverso de la cédula anterior en una FOTO de alta
/// resolución, además de en los frames del análisis.
///
/// Por qué (telemetría de pruebas reales): en 127 frames con el reverso en
/// cuadro ML Kit no DETECTÓ ningún código. El frame de análisis (1080p)
/// recortado al recuadro deja la tarjeta en ~940 px de ancho, y las barras
/// del PDF417 quedan por debajo de lo que el lector necesita.
///
/// Cada foto: en el hilo principal se cambia el análisis por la foto (ver
/// `EnlazadorCamara`: con los tres casos de uso a la vez la foto salía en
/// 1080 px), se saca, y apenas llega se vuelve a poner el análisis; recién
/// después se decodifica y se busca el código, en un hilo propio. El
/// resultado se entrega en `entregarEn` (el hilo del analizador), así queda
/// serializado con las lecturas de los frames.
///
/// Se dispara sólo cuando el texto ya dice que se ve el reverso de la
/// cédula anterior (ver `PlanificadorLectores.hayPistaReverso`), una foto a
/// la vez y como mucho una cada [intervaloMs]: cada una congela el preview
/// y pausa el texto un momento. La foto nunca se guarda: se libera apenas
/// ML Kit termina, y los bytes del código se tratan igual que en los frames
/// (`leerCodigos`: sólo el prefijo va a Rust y todo se pone en cero).
class FotografoPdf417(
    private val captura: ImageCapture,
    private val lector: BarcodeScanner,
    private val entregarEn: Executor,
    private val principal: Executor,
    private val enlazador: () -> EnlazadorCamara?,
    private val sesionActiva: AtomicBoolean,
    private val metricas: MetricasOcr,
    private val intervaloMs: Long = INTERVALO_ENTRE_FOTOS_MS,
    private val reloj: () -> Long = SystemClock::elapsedRealtime,
) {
    private val hilo: ExecutorService = Executors.newSingleThreadExecutor()

    /// Nunca rechaza: si el hilo ya se cerró (la pantalla se fue), la tarea
    /// corre en el lugar. Un rechazo dentro de un listener de ML Kit tumbaba
    /// la app (RejectedExecutionException en el hilo principal, Sentry
    /// CONTROL-ACCESO-MOBILE-1) al salir de la pantalla con una foto en
    /// curso.
    private val ejecutorFoto = Executor { tarea ->
        try {
            hilo.execute(tarea)
        } catch (_: RejectedExecutionException) {
            tarea.run()
        }
    }
    private val enCurso = AtomicBoolean(false)

    @Volatile
    private var ultimaFotoMs: Long? = null

    /// Saca una foto si corresponde; `onDatos` recibe la cédula si el
    /// código se leyó. Se puede llamar desde cualquier hilo.
    fun intentar(onDatos: (DatosPdf417Cedula) -> Unit) {
        val ahora = reloj()
        val ultima = ultimaFotoMs
        if (!sesionActiva.get() || (ultima != null && ahora - ultima < intervaloMs)) return
        if (!enCurso.compareAndSet(false, true)) return
        ultimaFotoMs = ahora
        principal.execute { fotografiar(onDatos) }
    }

    fun liberar() {
        hilo.shutdown()
    }

    /// En el hilo principal.
    private fun fotografiar(onDatos: (DatosPdf417Cedula) -> Unit) {
        val enlace = enlazador()
        if (!sesionActiva.get() || enlace == null) {
            enCurso.set(false)
            return
        }
        val inicio = System.nanoTime()
        val restaurar = restaurarUnaVez(enlace)
        try {
            enlace.usarFoto(captura)
            captura.takePicture(
                ejecutorFoto,
                object : ImageCapture.OnImageCapturedCallback() {
                    override fun onCaptureSuccess(imagen: ImageProxy) {
                        principal.execute(restaurar)
                        analizar(imagen, inicio, onDatos)
                    }

                    override fun onError(exception: ImageCaptureException) {
                        principal.execute(restaurar)
                        metricas.registrarFotoPdf417(ResultadoFotoPdf417.ERROR_CAPTURA, msDesde(inicio), anchoPx = null, detectados = 0)
                        enCurso.set(false)
                    }
                },
            )
        } catch (_: RuntimeException) {
            // Combinación no admitida o cámara ya liberada: se vuelve al
            // análisis y el escaneo sigue como sin foto.
            restaurar.run()
            metricas.registrarFotoPdf417(ResultadoFotoPdf417.ERROR_CAPTURA, msDesde(inicio), anchoPx = null, detectados = 0)
            enCurso.set(false)
        }
    }

    /// Vuelve a poner el análisis, una sola vez aunque se pida desde dos
    /// caminos, y sólo si la pantalla sigue abierta (si no, `liberar` de
    /// `EstadoCamaraOcr` ya soltó todo y reenlazar dejaría la cámara
    /// prendida). En el hilo principal.
    private fun restaurarUnaVez(enlace: EnlazadorCamara): Runnable {
        val hecho = AtomicBoolean(false)
        return Runnable {
            if (hecho.compareAndSet(false, true) && sesionActiva.get()) {
                try {
                    enlace.volverAlAnalisis(captura)
                } catch (_: RuntimeException) {
                    metricas.registrarFotoPdf417(ResultadoFotoPdf417.ERROR_ENLACE, 0, anchoPx = null, detectados = 0)
                }
            }
        }
    }

    private fun analizar(imagen: ImageProxy, inicio: Long, onDatos: (DatosPdf417Cedula) -> Unit) {
        if (!sesionActiva.get()) {
            imagen.close()
            enCurso.set(false)
            return
        }
        val rotacion = imagen.imageInfo.rotationDegrees
        val anchoPx = if (rotacion % 180 != 0) imagen.height else imagen.width
        val mapa = try {
            imagen.toBitmap()
        } catch (_: RuntimeException) {
            null
        } catch (_: OutOfMemoryError) {
            // Una foto grande es prescindible: sin memoria se sigue sin ella.
            null
        } finally {
            imagen.close()
        }
        if (mapa == null) {
            metricas.registrarFotoPdf417(ResultadoFotoPdf417.ERROR_LECTOR, msDesde(inicio), anchoPx, detectados = 0)
            enCurso.set(false)
            return
        }
        val tarea = try {
            lector.process(InputImage.fromBitmap(mapa, rotacion))
        } catch (_: RuntimeException) {
            // Lector ya cerrado: la pantalla se fue.
            mapa.recycle()
            enCurso.set(false)
            return
        }
        tarea.addOnCompleteListener(ejecutorFoto) {
            mapa.recycle()
            val codigos = tarea.takeIf { it.isSuccessful }?.result
            val ms = msDesde(inicio)
            // `entregarEn` tampoco rechaza (ver `EstadoCamaraOcr.ejecutorProcesamiento`).
            entregarEn.execute {
                try {
                    if (!sesionActiva.get()) {
                        codigos?.forEach { it.rawBytes?.fill(0) }
                        return@execute
                    }
                    if (codigos == null) {
                        metricas.registrarErrorLectorCodigo()
                        metricas.registrarFotoPdf417(ResultadoFotoPdf417.ERROR_LECTOR, ms, anchoPx, detectados = 0)
                        return@execute
                    }
                    val datos = leerCodigos(codigos, anchoPx, metricas)
                    val resultado = when {
                        datos != null -> ResultadoFotoPdf417.LEIDA
                        codigos.isNotEmpty() -> ResultadoFotoPdf417.CODIGO_INVALIDO
                        else -> ResultadoFotoPdf417.SIN_CODIGO
                    }
                    metricas.registrarFotoPdf417(resultado, ms, anchoPx, codigos.size)
                    if (datos != null) onDatos(datos)
                } finally {
                    enCurso.set(false)
                }
            }
        }
    }

    private fun msDesde(inicio: Long) = (System.nanoTime() - inicio) / 1_000_000
}

/// Cómo terminó una foto del PDF417 (sólo para la telemetría).
enum class ResultadoFotoPdf417 { LEIDA, CODIGO_INVALIDO, SIN_CODIGO, ERROR_CAPTURA, ERROR_LECTOR, ERROR_ENLACE }

/// Entre foto y foto: cada una congela el preview y pausa el texto un
/// momento; además da tiempo a que la persona acomode la tarjeta y no
/// calienta el teléfono si el código no se deja leer.
const val INTERVALO_ENTRE_FOTOS_MS = 2_000L
