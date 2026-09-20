package com.brisas.controlacceso

/// Rectángulo de enteros sin depender de `android.graphics.Rect` -- ese tipo
/// no se puede instanciar en tests unitarios de JVM sin Robolectric (el
/// stub de Android tira "not mocked"), y toda la aritmética de acá no
/// necesita nada de Android, sólo enteros.
data class RectanguloEntero(val left: Int, val top: Int, val right: Int, val bottom: Int) {
    val width: Int get() = right - left
    val height: Int get() = bottom - top
}

/// Misma región que dibuja `MarcoGuiaCedula` (84% del ancho, proporción de
/// cédula ISO/IEC 7810 ID-1 ≈ 1.586:1, centrada, con el mismo
/// desplazamiento vertical) -- pero en enteros de píxeles de imagen, no en
/// los `Float` de un `Canvas` de Compose. Los tres números de acá deben
/// mantenerse iguales a los de `MarcoGuiaCedula.kt` a mano (viven en
/// sistemas de coordenadas distintos, no se pueden compartir el mismo
/// código sin acoplar el dibujo de UI con el recorte para OCR) -- si se
/// ajusta uno, ajustar el otro.
object RegionGuiaOcr {
    const val FRACCION_ANCHO = 0.84f
    const val PROPORCION_ANCHO_ALTO = 1.586f
    const val FRACCION_TOP_CENTRO = 0.52f

    fun rectanguloEnPixeles(anchoVisible: Int, altoVisible: Int): RectanguloEntero {
        val ancho = (anchoVisible * FRACCION_ANCHO).toInt().coerceAtLeast(1)
        val alto = (ancho / PROPORCION_ANCHO_ALTO).toInt().coerceAtLeast(1)
        val left = (anchoVisible - ancho) / 2
        val top = (altoVisible * FRACCION_TOP_CENTRO - alto / 2f).toInt()
        return RectanguloEntero(
            left = left.coerceIn(0, anchoVisible),
            top = top.coerceIn(0, altoVisible),
            right = (left + ancho).coerceIn(0, anchoVisible),
            bottom = (top + alto).coerceIn(0, altoVisible),
        )
    }
}

/// Arma un byte array NV21 (plano Y completo + planos U/V intercalados como
/// VU) a partir de los planos crudos de una imagen YUV_420_888 -- formato
/// que entrega CameraX en `ImageAnalysis` frame a frame. Necesario porque
/// `android.graphics.YuvImage` (la única clase de Android que sabe recortar
/// + comprimir YUV sin decodificar antes a Bitmap) sólo acepta NV21, nunca
/// YUV_420_888 crudo.
///
/// Puro -- sólo bytes y enteros, nada de `android.media.Image` -- para
/// poder probar la conversión con datos sintéticos, sin cámara real ni
/// Robolectric. `yRowStride`/`uvRowStride`/`uvPixelStride` importan porque
/// ninguno de los tres está garantizado: el plano Y puede traer relleno al
/// final de cada fila (`rowStride` > ancho real), y las muestras de croma
/// pueden no ser contiguas (`pixelStride` > 1) -- copiar ignorando esto
/// produce una imagen corrida/basura en cualquier dispositivo cuyo HAL de
/// cámara no entregue los planos ya empaquetados.
fun construirNv21(
    ancho: Int,
    alto: Int,
    y: ByteArray,
    yRowStride: Int,
    u: ByteArray,
    v: ByteArray,
    uvRowStride: Int,
    uvPixelStride: Int,
): ByteArray {
    val nv21 = ByteArray(ancho * alto + (ancho * alto) / 2)
    var posicion = 0
    for (fila in 0 until alto) {
        val inicio = fila * yRowStride
        System.arraycopy(y, inicio, nv21, posicion, ancho)
        posicion += ancho
    }
    val anchoUv = ancho / 2
    val altoUv = alto / 2
    for (fila in 0 until altoUv) {
        for (columna in 0 until anchoUv) {
            val indice = fila * uvRowStride + columna * uvPixelStride
            nv21[posicion++] = v[indice]
            nv21[posicion++] = u[indice]
        }
    }
    return nv21
}
