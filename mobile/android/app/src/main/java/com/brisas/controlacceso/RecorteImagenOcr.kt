package com.brisas.controlacceso

import java.nio.ByteBuffer

/// Rectángulo de enteros sin depender de `android.graphics.Rect` -- ese tipo
/// no se puede instanciar en tests unitarios de JVM sin Robolectric (el
/// stub de Android tira "not mocked"), y toda la aritmética de acá no
/// necesita nada de Android, sólo enteros.
data class RectanguloEntero(val left: Int, val top: Int, val right: Int, val bottom: Int) {
    val width: Int get() = right - left
    val height: Int get() = bottom - top
}

/// Qué parte de lo visible (la imagen YA rotada como la ve quien opera)
/// se le entrega a ML Kit. [RegionGuiaOcr] es el recuadro guía que se
/// dibuja; [SubregionRecorte] es una parte de él (p. ej. sólo la banda del
/// MRZ, ver `SeguidorBandaMrz`).
interface RegionRecorte {
    fun rectanguloEnPixeles(anchoVisible: Int, altoVisible: Int): RectanguloEntero
}

/// Rectángulo expresado en fracciones (0..1) del ancho y alto de otra
/// imagen. Así las cajas de ML Kit, que llegan en píxeles del recorte
/// analizado, se pueden llevar al recuadro guía sin depender de la
/// resolución.
data class FraccionesRect(val izquierda: Float, val arriba: Float, val derecha: Float, val abajo: Float) {
    init {
        require(izquierda < derecha && arriba < abajo) { "Rectángulo vacío: $this" }
    }

    val ancho: Float get() = derecha - izquierda
    val alto: Float get() = abajo - arriba

    companion object {
        /// Caja en píxeles (`left`/`top`/`right`/`bottom`) de una imagen
        /// de `anchoImagen` x `altoImagen`, llevada a fracciones y recortada
        /// a la imagen. `null` si queda vacía.
        fun desdePixeles(left: Int, top: Int, right: Int, bottom: Int, anchoImagen: Int, altoImagen: Int): FraccionesRect? {
            if (anchoImagen <= 0 || altoImagen <= 0) return null
            val izq = (left.toFloat() / anchoImagen).coerceIn(0f, 1f)
            val der = (right.toFloat() / anchoImagen).coerceIn(0f, 1f)
            val arr = (top.toFloat() / altoImagen).coerceIn(0f, 1f)
            val aba = (bottom.toFloat() / altoImagen).coerceIn(0f, 1f)
            return if (izq < der && arr < aba) FraccionesRect(izq, arr, der, aba) else null
        }
    }
}

/// La parte `fracciones` del rectángulo de `base`.
data class SubregionRecorte(val base: RegionRecorte, val fracciones: FraccionesRect) : RegionRecorte {
    override fun rectanguloEnPixeles(anchoVisible: Int, altoVisible: Int): RectanguloEntero {
        val r = base.rectanguloEnPixeles(anchoVisible, altoVisible)
        return RectanguloEntero(
            left = r.left + (r.width * fracciones.izquierda).toInt(),
            top = r.top + (r.height * fracciones.arriba).toInt(),
            right = r.left + (r.width * fracciones.derecha).toInt(),
            bottom = r.top + (r.height * fracciones.abajo).toInt(),
        )
    }
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
) : RegionRecorte {
    override fun rectanguloEnPixeles(anchoVisible: Int, altoVisible: Int): RectanguloEntero {
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

        // Gafete VERTICAL (In House, KOF) sostenido en la pantalla de
        // escaneo general, cuyo recuadro es de tarjeta horizontal: el
        // gafete no entra entero y la franja "CONTRATISTA" / "COSTA RICA"
        // queda debajo del recuadro -- peor con estuche, que lo agranda.
        // Mismo ancho que `TARJETA_ID`, pero más alta que ancha, centrada
        // en el mismo punto. La usa `ControladorEncuadre` (ver
        // `EncuadreAdaptativo.kt`): como sonda sin dibujar y, cuando se
        // reconoce un documento vertical, como recuadro visible.
        val GAFETE_VERTICAL = RegionGuiaOcr(fraccionAncho = 0.84f, proporcionAnchoAlto = 0.75f, fraccionTopCentro = 0.52f)
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
fun rectanguloEnSensor(crop: RectanguloEntero, rotacionGrados: Int, region: RegionRecorte): RectanguloEntero {
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
/// Lee DIRECTO de los `ByteBuffer` de cada plano, fila por fila y sólo
/// dentro de `rect` (auditoría OCR 2026-09-28). Antes se copiaban los tres
/// planos ENTEROS a arreglos (~4 MB por frame a 1080p, retenidos además
/// entre frames en `BuffersOcrReutilizables`) y recién ahí se recortaba --
/// el recuadro guía es cerca de un cuarto del frame, así que casi todo lo
/// copiado se tiraba. Cada búfer se lee sobre un `duplicate()`: no mueve
/// la posición del original, así el fallback `InputImage.fromMediaImage`
/// sigue encontrando los planos intactos. Las posiciones se cuentan desde
/// la posición inicial de cada búfer (lo mismo que leía la copia anterior).
///
/// Puro -- sólo búferes y enteros, nada de `android.media.Image` -- para
/// poder probarlo con datos sintéticos. `yRowStride`/`uvRowStride`/
/// `uvPixelStride` importan porque ninguno está garantizado: el plano Y
/// puede traer relleno al final de cada fila y las muestras de croma
/// pueden no ser contiguas (`pixelStride` > 1).
fun recortarYuvANv21(
    rect: RectanguloEntero,
    y: ByteBuffer,
    yRowStride: Int,
    u: ByteBuffer,
    v: ByteBuffer,
    uvRowStride: Int,
    uvPixelStride: Int,
): ByteArray {
    val ancho = rect.width
    val alto = rect.height
    require(ancho > 0 && alto > 0 && ancho % 2 == 0 && alto % 2 == 0) {
        "El recorte NV21 necesita ancho y alto pares y positivos: ${ancho}x$alto"
    }
    val nv21 = ByteArray(ancho * alto + (ancho * alto) / 2)

    val lecturaY = y.duplicate()
    val baseY = lecturaY.position()
    for (fila in 0 until alto) {
        lecturaY.position(baseY + (rect.top + fila) * yRowStride + rect.left)
        lecturaY.get(nv21, fila * ancho, ancho)
    }

    val anchoCroma = ancho / 2
    // Bytes que ocupa una fila de croma del recorte dentro del plano: hasta
    // la última muestra inclusive, no `anchoCroma * pixelStride` -- la
    // última fila de un plano con `pixelStride` 2 no trae el byte de relleno
    // final y leerlo se saldría del búfer.
    val largoFilaCroma = (anchoCroma - 1) * uvPixelStride + 1
    val filaU = ByteArray(largoFilaCroma)
    val filaV = ByteArray(largoFilaCroma)
    val lecturaU = u.duplicate()
    val lecturaV = v.duplicate()
    val baseU = lecturaU.position()
    val baseV = lecturaV.position()
    val columnaUv = rect.left / 2
    val filaUv = rect.top / 2
    var posicion = ancho * alto
    for (fila in 0 until alto / 2) {
        val desplazamiento = (filaUv + fila) * uvRowStride + columnaUv * uvPixelStride
        lecturaU.position(baseU + desplazamiento)
        lecturaU.get(filaU)
        lecturaV.position(baseV + desplazamiento)
        lecturaV.get(filaV)
        for (columna in 0 until anchoCroma) {
            val indice = columna * uvPixelStride
            nv21[posicion++] = filaV[indice]
            nv21[posicion++] = filaU[indice]
        }
    }
    return nv21
}
