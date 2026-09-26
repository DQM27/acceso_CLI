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

/// Lleva el recuadro guía de [RegionGuiaOcr] -- expresado como la pantalla,
/// YA rotado -- a coordenadas del sensor SIN rotar, dentro de `crop` (el
/// `cropRect` de CameraX, lo visible en pantalla, en coordenadas del
/// sensor). Así se recorta directo sobre los planos YUV que entrega la
/// cámara y ML Kit recibe sólo esa región en NV21, rotada por él mismo --
/// en vez de convertir el frame ENTERO a ARGB, armar un bitmap, rotarlo
/// completo y recién ahí recortar (4 bitmaps grandes por frame, ver
/// historial de `recortarParaOcr`).
///
/// `rotacionGrados` es la de `ImageInfo.rotationDegrees`: cuánto hay que
/// girar el sensor en sentido horario para verlo derecho. Girar 90° lleva
/// el punto (x, y) del sensor (ancho W, alto H) a (H-1-y, x) en la imagen
/// derecha; se invierte eso para cada una de las 4 rotaciones posibles.
///
/// El resultado queda alineado a coordenadas PARES: en NV21 el croma
/// viene submuestreado de a bloques de 2x2, un borde impar partiría un
/// bloque.
fun rectanguloEnSensor(crop: RectanguloEntero, rotacionGrados: Int, region: RegionGuiaOcr): RectanguloEntero {
    val w = crop.width
    val h = crop.height
    val rotada = rotacionGrados % 180 != 0
    val r = region.rectanguloEnPixeles(if (rotada) h else w, if (rotada) w else h)
    val enCrop = when (rotacionGrados) {
        90 -> RectanguloEntero(left = r.top, top = h - r.right, right = r.bottom, bottom = h - r.left)
        180 -> RectanguloEntero(left = w - r.right, top = h - r.bottom, right = w - r.left, bottom = h - r.top)
        270 -> RectanguloEntero(left = w - r.bottom, top = r.left, right = w - r.top, bottom = r.right)
        else -> r
    }
    val par = 1.inv()
    return RectanguloEntero(
        left = (crop.left + enCrop.left) and par,
        top = (crop.top + enCrop.top) and par,
        right = (crop.left + enCrop.right) and par,
        bottom = (crop.top + enCrop.bottom) and par,
    )
}

/// Copia sólo `rect` (en coordenadas del sensor, alineado a pares -- ver
/// [rectanguloEnSensor]) de los planos crudos de una imagen YUV_420_888 a
/// un byte array NV21 (plano Y + croma intercalado VU), el formato que
/// `InputImage.fromByteArray` acepta directo. Ningún paso de color: ML Kit
/// lee la luminancia tal cual la entrega el sensor. Antes se convertía el
/// frame completo a ARGB con coeficientes de rango TV aplicados sin restar
/// el offset de 16 (la imagen salía ~16% más clara y todo lo que tuviera
/// Y >= ~220 se saturaba a blanco -- justo los reflejos de la cédula
/// plastificada, donde el texto ya cuesta leer).
///
/// Puro -- sólo bytes y enteros, nada de `android.media.Image` -- para
/// poder probarlo con datos sintéticos. `yRowStride`/`uvRowStride`/
/// `uvPixelStride` importan porque ninguno está garantizado: el plano Y
/// puede traer relleno al final de cada fila y las muestras de croma
/// pueden no ser contiguas (`pixelStride` > 1).
fun recortarYuvANv21(
    rect: RectanguloEntero,
    y: ByteArray,
    yRowStride: Int,
    u: ByteArray,
    v: ByteArray,
    uvRowStride: Int,
    uvPixelStride: Int,
): ByteArray {
    val ancho = rect.width
    val alto = rect.height
    val nv21 = ByteArray(ancho * alto + (ancho * alto) / 2)
    for (fila in 0 until alto) {
        System.arraycopy(y, (rect.top + fila) * yRowStride + rect.left, nv21, fila * ancho, ancho)
    }
    var posicion = ancho * alto
    val columnaUv = rect.left / 2
    val filaUv = rect.top / 2
    for (fila in 0 until alto / 2) {
        val inicio = (filaUv + fila) * uvRowStride
        for (columna in 0 until ancho / 2) {
            val indice = inicio + (columnaUv + columna) * uvPixelStride
            nv21[posicion++] = v[indice]
            nv21[posicion++] = u[indice]
        }
    }
    return nv21
}

/// Reune los buffers donde se copian los planos crudos de cada frame
/// (`yBytes`/`uBytes`/`vBytes`) para reusarlos entre frames en vez de
/// asignarlos desde cero cada vez (MV-07, auditoría de rendimiento
/// 2026-09-25) -- con resolución fija (ver `construirAnalizadorOcr`) el
/// tamaño no cambia dentro de una sesión de escaneo. El recorte NV21 que
/// recibe ML Kit NO se reusa: `recognizer.process` lo lee de forma
/// asíncrona, después de que el analizador ya soltó el frame.
///
/// Una instancia por apertura de pantalla, igual que `EstabilizadorLectura`
/// -- pensada para el único hilo del analizador de cámara
/// (`ejecutorAnalisis`).
class BuffersOcrReutilizables {
    private var yBytes: ByteArray? = null
    private var uBytes: ByteArray? = null
    private var vBytes: ByteArray? = null

    fun yBytes(tamano: Int): ByteArray = reusarByteArray(yBytes, tamano) { yBytes = it }
    fun uBytes(tamano: Int): ByteArray = reusarByteArray(uBytes, tamano) { uBytes = it }
    fun vBytes(tamano: Int): ByteArray = reusarByteArray(vBytes, tamano) { vBytes = it }

    private inline fun reusarByteArray(actual: ByteArray?, tamano: Int, guardar: (ByteArray) -> Unit): ByteArray {
        if (actual != null && actual.size == tamano) return actual
        return ByteArray(tamano).also(guardar)
    }
}
