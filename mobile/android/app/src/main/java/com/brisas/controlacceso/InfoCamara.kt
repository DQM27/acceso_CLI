package com.brisas.controlacceso

import android.annotation.SuppressLint
import android.graphics.ImageFormat
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraMetadata
import android.util.Size
import androidx.camera.camera2.interop.Camera2CameraInfo
import androidx.camera.camera2.interop.ExperimentalCamera2Interop
import androidx.camera.core.Camera
import androidx.camera.core.CameraState
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.Observer

/// Qué puede dar la cámara, para saber si se puede pedir más resolución al
/// análisis del PDF417 (evento `camara_info` de la telemetría de diagnóstico,
/// una vez por apertura de cámara). Sólo números y nombres de niveles; nada
/// de la persona ni de lo que se escanea.

/// Nombre del nivel de hardware de Camera2 (`INFO_SUPPORTED_HARDWARE_LEVEL`).
/// `LEGACY` = poco control manual, `LIMITED` < `FULL` < `LEVEL_3`.
fun nombreNivelHardware(nivel: Int?): String = when (nivel) {
    null -> "desconocido"
    CameraMetadata.INFO_SUPPORTED_HARDWARE_LEVEL_LEGACY -> "LEGACY"
    CameraMetadata.INFO_SUPPORTED_HARDWARE_LEVEL_LIMITED -> "LIMITED"
    CameraMetadata.INFO_SUPPORTED_HARDWARE_LEVEL_FULL -> "FULL"
    CameraMetadata.INFO_SUPPORTED_HARDWARE_LEVEL_3 -> "LEVEL_3"
    CameraMetadata.INFO_SUPPORTED_HARDWARE_LEVEL_EXTERNAL -> "EXTERNAL"
    else -> "otro_$nivel"
}

/// El tamaño (ancho, alto) de más píxeles; `null` si no hay ninguno.
fun mayorTamano(tamanos: List<Pair<Int, Int>>): Pair<Int, Int>? =
    tamanos.maxByOrNull { (ancho, alto) -> ancho.toLong() * alto }

/// Distancia mínima de enfoque en centímetros a partir de dioptrías
/// (`LENS_INFO_MINIMUM_FOCUS_DISTANCE`, 1/metros). 0 dioptrías = enfoque
/// fijo (sin enfoque cercano) y sin dato no hay distancia: `null` en ambos.
fun distanciaMinimaEnfoqueCm(dioptrias: Float?): Float? =
    if (dioptrias == null || dioptrias <= 0f) null else 100f / dioptrias

/// Reporta `camara_info` cuando la cámara ya está abierta (antes de eso
/// `resolutionInfo` de los casos de uso aún no existe). Una sola vez por
/// llamada; sin efecto si la telemetría está apagada. Corre en el hilo
/// principal (el observador de `cameraState` lo exige) y sólo lee datos ya
/// disponibles: no toca el flujo de frames.
@SuppressLint("UnsafeOptInUsageError")
@androidx.annotation.OptIn(ExperimentalCamera2Interop::class)
fun reportarCamaraInfo(
    camara: Camera,
    analisis: ImageAnalysis,
    preview: Preview,
    lifecycleOwner: LifecycleOwner,
    // Datos que se suman al evento (ej. `modo_codigo` al cambiar al análisis
    // de alta resolución del PDF417).
    extra: Map<String, Any?> = emptyMap(),
) {
    if (!Telemetria.activa) return
    val estado = camara.cameraInfo.cameraState
    var reportado = false
    lateinit var observador: Observer<CameraState>
    observador = Observer { cambio ->
        if (reportado || cambio.type != CameraState.Type.OPEN) return@Observer
        reportado = true
        estado.removeObserver(observador)
        try {
            Telemetria.evento("camara_info", datosCamaraInfo(camara, analisis, preview) + extra)
        } catch (_: RuntimeException) {
            // Un dato que la cámara no da nunca debe romper el escaneo.
        }
    }
    estado.observe(lifecycleOwner, observador)
}

@SuppressLint("UnsafeOptInUsageError")
@androidx.annotation.OptIn(ExperimentalCamera2Interop::class)
private fun datosCamaraInfo(camara: Camera, analisis: ImageAnalysis, preview: Preview): Map<String, Any?> {
    val info = Camera2CameraInfo.from(camara.cameraInfo)
    val nivel = info.getCameraCharacteristic(CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL)
    val sensor: Size? = info.getCameraCharacteristic(CameraCharacteristics.SENSOR_INFO_PIXEL_ARRAY_SIZE)
    val mapa = info.getCameraCharacteristic(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP)
    fun tamanos(lista: Array<Size>?) = lista.orEmpty().map { it.width to it.height }
    val yuv = mayorTamano(tamanos(mapa?.getOutputSizes(ImageFormat.YUV_420_888)))
    val yuvAltaResolucion = mayorTamano(tamanos(mapa?.getHighResolutionOutputSizes(ImageFormat.YUV_420_888)))
    val jpeg = mayorTamano(tamanos(mapa?.getOutputSizes(ImageFormat.JPEG)))
    val enfoque = info.getCameraCharacteristic(CameraCharacteristics.LENS_INFO_MINIMUM_FOCUS_DISTANCE)
    val resolucionAnalisis = analisis.resolutionInfo?.resolution
    val resolucionPreview = preview.resolutionInfo?.resolution
    return mapOf(
        "pantalla" to Telemetria.pantallaActual(),
        "hardware_nivel" to nombreNivelHardware(nivel),
        "analisis_ancho" to resolucionAnalisis?.width,
        "analisis_alto" to resolucionAnalisis?.height,
        "preview_ancho" to resolucionPreview?.width,
        "preview_alto" to resolucionPreview?.height,
        "yuv_max_ancho" to yuv?.first,
        "yuv_max_alto" to yuv?.second,
        // Tamaños que sólo se dan a menor velocidad (sensores de 50 MP): si
        // no hay, el YUV máximo es el techo.
        "yuv_alta_resolucion_max_ancho" to yuvAltaResolucion?.first,
        "yuv_alta_resolucion_max_alto" to yuvAltaResolucion?.second,
        "jpeg_max_ancho" to jpeg?.first,
        "jpeg_max_alto" to jpeg?.second,
        "sensor_ancho" to sensor?.width,
        "sensor_alto" to sensor?.height,
        "zoom_max" to camara.cameraInfo.zoomState.value?.maxZoomRatio,
        "enfoque_min_dioptrias" to enfoque,
        "enfoque_min_cm" to distanciaMinimaEnfoqueCm(enfoque),
    )
}
