package com.brisas.controlacceso

/// Rectángulo de enteros sin depender de `android.graphics.Rect` -- ese tipo
/// no se puede instanciar en tests unitarios de JVM sin Robolectric (el
/// stub de Android tira "not mocked"), y toda la aritmética de acá no
/// necesita nada de Android, sólo enteros.
data class RectanguloEntero(val left: Int, val top: Int, val right: Int, val bottom: Int) {
    val width: Int get() = right - left
    val height: Int get() = bottom - top
}

/// Región de interés para recortar antes del OCR -- misma forma que
/// `MarcoGuiaCedula` debe dibujar para esa pantalla (fracción del ancho
/// visible, proporción ancho:alto, y a qué fracción de la altura queda el
/// centro vertical). En enteros de píxeles de imagen, no en los `Float` de
/// un `Canvas` de Compose -- `MarcoGuiaCedula.kt` recibe estos mismos tres
/// números por parámetro para que el recuadro que ve la persona y lo que
/// de verdad se analiza sean SIEMPRE la misma región (una sola fuente de
/// verdad por pantalla, no dos copias que se puedan desalinear).
///
/// `TARJETA_ID` (cédula, gafete, carnet KOF) -- proporción real de una
/// cédula/tarjeta ISO/IEC 7810 ID-1.
///
/// `COMPROBANTE_RUTA` -- segundo intento para el comprobante de carga de
/// ruta (2026-09-20, pedido explícito del usuario tras revertir el primer
/// intento a `region = null`). Los dos intentos anteriores (forzar
/// horizontal, después ensanchar en vertical) adivinaban una región
/// AJUSTADA al documento y terminaron dejando la pantalla sin leer ningún
/// campo -- ver el historial de `PantallaEscanearComprobanteRuta.kt`. Este
/// intento va deliberadamente holgado en vez de ajustado: recorta sólo
/// ~10% de cada dimensión del frame (95% del ancho, ~90% del alto de un
/// frame ya rotado a 720x1280 -- ver `recortarParaOcr`), lo suficiente para
/// seguir la recomendación de ML Kit de no mandarle el frame entero sin
/// arriesgarse a repetir el mismo fallo de cortar el campo que se necesita
/// leer. Pendiente de validar contra el dispositivo real, igual que los
/// intentos anteriores.
data class RegionGuiaOcr(
    val fraccionAncho: Float,
    val proporcionAnchoAlto: Float,
    val fraccionTopCentro: Float,
) {
    fun rectanguloEnPixeles(anchoVisible: Int, altoVisible: Int): RectanguloEntero {
        val ancho = (anchoVisible * fraccionAncho).toInt().coerceAtLeast(1)
        val alto = (ancho / proporcionAnchoAlto).toInt().coerceAtLeast(1)
        val left = (anchoVisible - ancho) / 2
        val top = (altoVisible * fraccionTopCentro - alto / 2f).toInt()
        return RectanguloEntero(
            left = left.coerceIn(0, anchoVisible),
            top = top.coerceIn(0, altoVisible),
            right = (left + ancho).coerceIn(0, anchoVisible),
            bottom = (top + alto).coerceIn(0, altoVisible),
        )
    }

    companion object {
        val TARJETA_ID = RegionGuiaOcr(fraccionAncho = 0.84f, proporcionAnchoAlto = 1.586f, fraccionTopCentro = 0.52f)
        val COMPROBANTE_RUTA = RegionGuiaOcr(fraccionAncho = 0.95f, proporcionAnchoAlto = 0.59f, fraccionTopCentro = 0.5f)
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
