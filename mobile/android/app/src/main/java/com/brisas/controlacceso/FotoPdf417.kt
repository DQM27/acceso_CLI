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
/// del PDF417 quedan por debajo de lo que el lector necesita. Una foto de
/// ~12 MP tiene unas 3,5 veces más píxeles por barra.
///
/// Se dispara sólo cuando el texto ya dice que se ve el reverso de la
/// cédula anterior (ver `PlanificadorLectores.hayPistaReverso`), una foto a
/// la vez y como mucho una cada [intervaloMs]. Decodificar la foto y
/// buscar el código corre en un hilo propio para no frenar el análisis; el
/// resultado se entrega en `entregarEn` (el hilo del analizador), así queda
/// serializado con las lecturas de los frames. La foto nunca se guarda: se
/// libera apenas ML Kit termina, y los bytes del código se tratan igual
/// que en los frames (`leerCodigos`: sólo el prefijo va a Rust y todo se
/// pone en cero).
class FotografoPdf417(
    private val captura: ImageCapture,
    private val lector: BarcodeScanner,
    private val entregarEn: Executor,
    private val sesionActiva: AtomicBoolean,
    private val metricas: MetricasOcr,
    private val intervaloMs: Long = INTERVALO_ENTRE_FOTOS_MS,
    private val reloj: () -> Long = SystemClock::elapsedRealtime,
) {
    private val hilo: ExecutorService = Executors.newSingleThreadExecutor()
    private val enCurso = AtomicBoolean(false)

    @Volatile
    private var ultimaFotoMs: Long? = null

    /// Saca una foto si corresponde; `onDatos` recibe la cédula si el
    /// código se leyó.
    fun intentar(onDatos: (DatosPdf417Cedula) -> Unit) {
        val ahora = reloj()
        val ultima = ultimaFotoMs
        if (!sesionActiva.get() || (ultima != null && ahora - ultima < intervaloMs)) return
        if (!enCurso.compareAndSet(false, true)) return
        ultimaFotoMs = ahora
        val inicio = System.nanoTime()
        try {
            captura.takePicture(
                hilo,
                object : ImageCapture.OnImageCapturedCallback() {
                    override fun onCaptureSuccess(imagen: ImageProxy) = analizar(imagen, inicio, onDatos)

                    override fun onError(exception: ImageCaptureException) {
                        metricas.registrarFotoPdf417(ResultadoFotoPdf417.ERROR_CAPTURA, msDesde(inicio), anchoPx = null, detectados = 0)
                        enCurso.set(false)
                    }
                },
            )
        } catch (_: RuntimeException) {
            // Cámara sin enlazar todavía (o ya liberada), o hilo cerrado.
            enCurso.set(false)
        }
    }

    fun liberar() {
        hilo.shutdown()
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
        tarea.addOnCompleteListener(hilo) {
            mapa.recycle()
            val codigos = tarea.takeIf { it.isSuccessful }?.result
            val ms = msDesde(inicio)
            try {
                entregarEn.execute {
                    try {
                        if (!sesionActiva.get()) return@execute
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
            } catch (_: RejectedExecutionException) {
                // El analizador ya cerró: nadie espera el resultado. Los
                // bytes crudos de los códigos se ponen en cero igual.
                codigos?.forEach { it.rawBytes?.fill(0) }
                enCurso.set(false)
            }
        }
    }

    private fun msDesde(inicio: Long) = (System.nanoTime() - inicio) / 1_000_000
}

/// Cómo terminó una foto del PDF417 (sólo para la telemetría).
enum class ResultadoFotoPdf417 { LEIDA, CODIGO_INVALIDO, SIN_CODIGO, ERROR_CAPTURA, ERROR_LECTOR }

/// Entre foto y foto: da tiempo a que la persona acomode la tarjeta y no
/// calienta el teléfono si el código no se deja leer.
const val INTERVALO_ENTRE_FOTOS_MS = 1_200L
