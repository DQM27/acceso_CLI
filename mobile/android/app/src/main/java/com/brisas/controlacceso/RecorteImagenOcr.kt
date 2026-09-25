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
/// `COMPROBANTE_RUTA` -- tercer intento para el comprobante de carga de
/// ruta (2026-09-20). Los dos primeros (forzar horizontal, después
/// ensanchar en vertical -- ver el historial de
/// `PantallaEscanearComprobanteRuta.kt`) adivinaban una región ajustada
/// SIN validar contra el dispositivo real y dejaron la pantalla sin leer
/// ningún campo; un cuarto intento después la volvió holgada (95%/90% del
/// frame) para confirmar primero que el recorte en sí no era el problema
/// -- ese sí funcionó. Con eso ya confirmado, este achica la región a una
/// escala parecida a `TARJETA_ID` (pedido explícito del usuario: "no hay
/// necesidad de esa enorme área", comparándola contra el recuadro angosto
/// de Carnet KOF/Vehículo): "Ruta/No.de Carga:" y "Transporte:" quedan
/// relativamente cerca entre sí en el documento real (ver
/// `LectorComprobanteRuta.kt`), no hace falta encuadrar la hoja completa
/// para que ambos entren. `fraccionTopCentro` igual a `TARJETA_ID` (0.52,
/// no 0.38 como en el ajuste anterior) -- pedido explícito del usuario: el
/// recuadro quedaba corrido hacia arriba en vez de centrado como el resto.
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
        val COMPROBANTE_RUTA = RegionGuiaOcr(fraccionAncho = 0.85f, proporcionAnchoAlto = 1.3f, fraccionTopCentro = 0.52f)
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

/// Convierte un NV21 (como el que arma [construirNv21]) directo a píxeles
/// ARGB_8888, sin pasar por JPEG -- reemplaza el camino anterior
/// `YuvImage.compressToJpeg` + `BitmapFactory.decodeByteArray` (auditoría de
/// rendimiento 2026-09-25: esa ida y vuelta comprimía a JPEG con pérdida y
/// después la descomprimía completa, el costo de CPU más alto por frame de
/// las 4 pantallas de escaneo, y de paso perdía calidad por la compresión
/// con pérdida -- innecesaria acá porque el resultado nunca se guarda ni se
/// muestra, sólo se le pasa a ML Kit).
///
/// Coeficientes BT.601 de rango completo (Y' 0-255, no el 16-235 "TV
/// range") -- los mismos que usa históricamente `YuvImage`/la mayoría de
/// HALs de cámara Android para este formato, para no introducir un cambio
/// de color perceptible frente al camino anterior.
///
/// Puro -- sólo bytes y enteros, nada de `android.graphics.Bitmap` -- para
/// poder probarlo con datos sintéticos, mismo motivo que [construirNv21].
/// El llamador arma el `Bitmap` con `Bitmap.createBitmap(pixeles, ancho,
/// alto, Config.ARGB_8888)`.
fun convertirNv21AArgb(nv21: ByteArray, ancho: Int, alto: Int): IntArray {
    val pixeles = IntArray(ancho * alto)
    val tamanoPlanoY = ancho * alto
    for (fila in 0 until alto) {
        var indiceY = fila * ancho
        var indiceUv = tamanoPlanoY + (fila shr 1) * ancho
        var u = 0
        var v = 0
        for (columna in 0 until ancho) {
            val y = (nv21[indiceY].toInt() and 0xff)
            if (columna and 1 == 0) {
                v = (nv21[indiceUv++].toInt() and 0xff) - 128
                u = (nv21[indiceUv++].toInt() and 0xff) - 128
            }
            val y1192 = 1192 * y
            val r = (y1192 + 1634 * v).coerceIn(0, 262143)
            val g = (y1192 - 833 * v - 400 * u).coerceIn(0, 262143)
            val b = (y1192 + 2066 * u).coerceIn(0, 262143)
            pixeles[indiceY] = -0x1000000 or
                ((r shl 6) and 0xff0000) or
                ((g shr 2) and 0xff00) or
                ((b shr 10) and 0xff)
            indiceY++
        }
    }
    return pixeles
}
