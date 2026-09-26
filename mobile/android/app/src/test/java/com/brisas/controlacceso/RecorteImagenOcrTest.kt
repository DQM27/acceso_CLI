package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class RecorteImagenOcrTest {

    @Test
    fun rectanguloGuiaQuedaCentradoYConLaProporcionDeCedula() {
        val rect = RegionGuiaOcr.TARJETA_ID.rectanguloEnPixeles(anchoVisible = 1000, altoVisible = 2000)

        assertEquals(840, rect.width)
        // 840 / 1.586 truncado a entero.
        assertEquals(529, rect.height)
        // Centrado horizontalmente: mismo margen a ambos lados.
        assertEquals(rect.left, 1000 - rect.right)
        assertEquals(80, rect.left)
    }

    @Test
    fun rectanguloGuiaDelComprobanteQuedaCentradoYEnEscalaParecidaATarjeta() {
        // Frame ya rotado a proporción de pantalla (720x1280, como entrega
        // recortarParaOcr) -- región chica, escala comparable a TARJETA_ID
        // (ver el doc-comment de RegionGuiaOcr.COMPROBANTE_RUTA), no el
        // frame casi completo del intento anterior.
        val rect = RegionGuiaOcr.COMPROBANTE_RUTA.rectanguloEnPixeles(anchoVisible = 720, altoVisible = 1280)

        assertEquals(612, rect.width) // 85% de 720
        assertEquals(470, rect.height) // 612 / 1.3 truncado a entero
        assertEquals(rect.left, 720 - rect.right)
        assertEquals(54, rect.left)
        // Mismo fraccionTopCentro que TARJETA_ID (0.52) -- pedido explícito
        // del usuario: el recuadro quedaba corrido hacia arriba en un
        // ajuste anterior (0.38) en vez de centrado como el resto.
        assertEquals(430, rect.top)
    }

    @Test
    fun rectanguloGuiaNuncaSaleDeLaImagenAunConDimensionesChicas() {
        val rect = RegionGuiaOcr.TARJETA_ID.rectanguloEnPixeles(anchoVisible = 10, altoVisible = 10)

        assertEquals(true, rect.left in 0..10)
        assertEquals(true, rect.top in 0..10)
        assertEquals(true, rect.right in 0..10)
        assertEquals(true, rect.bottom in 0..10)
    }

    // --- rectanguloEnSensor: el recuadro de pantalla llevado al sensor ---

    /// Rota un punto (x, y) del sensor (ancho w, alto h) como lo verá la
    /// pantalla: `rotationDegrees` en sentido horario. Referencia
    /// independiente para comprobar el mapeo inverso.
    private fun rotarPunto(x: Int, y: Int, w: Int, h: Int, grados: Int): Pair<Int, Int> = when (grados) {
        90 -> (h - 1 - y) to x
        180 -> (w - 1 - x) to (h - 1 - y)
        270 -> y to (w - 1 - x)
        else -> x to y
    }

    private fun comprobarMapeo(grados: Int) {
        val (w, h) = 1920 to 1080
        val crop = RectanguloEntero(0, 0, w, h)
        val enSensor = rectanguloEnSensor(crop, grados, RegionGuiaOcr.TARJETA_ID)
        val (anchoRot, altoRot) = if (grados % 180 == 0) w to h else h to w
        val guia = RegionGuiaOcr.TARJETA_ID.rectanguloEnPixeles(anchoRot, altoRot)
        // Las 4 esquinas del recorte en el sensor, rotadas como las ve la
        // pantalla, caen (salvo el redondeo a pares) sobre el recuadro guía.
        val esquinas = listOf(
            enSensor.left to enSensor.top,
            (enSensor.right - 1) to enSensor.top,
            enSensor.left to (enSensor.bottom - 1),
            (enSensor.right - 1) to (enSensor.bottom - 1),
        ).map { (x, y) -> rotarPunto(x, y, w, h, grados) }
        val xs = esquinas.map { it.first }
        val ys = esquinas.map { it.second }
        assertTrue("x min ${xs.min()} vs ${guia.left} ($grados°)", kotlin.math.abs(xs.min() - guia.left) <= 2)
        assertTrue("x max ${xs.max()} vs ${guia.right - 1} ($grados°)", kotlin.math.abs(xs.max() - (guia.right - 1)) <= 2)
        assertTrue("y min ${ys.min()} vs ${guia.top} ($grados°)", kotlin.math.abs(ys.min() - guia.top) <= 2)
        assertTrue("y max ${ys.max()} vs ${guia.bottom - 1} ($grados°)", kotlin.math.abs(ys.max() - (guia.bottom - 1)) <= 2)
        assertTrue(enSensor.left % 2 == 0 && enSensor.top % 2 == 0 && enSensor.width % 2 == 0 && enSensor.height % 2 == 0)
        assertTrue(enSensor.left >= 0 && enSensor.top >= 0 && enSensor.right <= w && enSensor.bottom <= h)
    }

    @Test
    fun rectanguloEnSensorSinRotacion() = comprobarMapeo(0)

    @Test
    fun rectanguloEnSensorRotado90ComoLaCamaraTraseraEnVertical() = comprobarMapeo(90)

    @Test
    fun rectanguloEnSensorRotado180() = comprobarMapeo(180)

    @Test
    fun rectanguloEnSensorRotado270() = comprobarMapeo(270)

    @Test
    fun rectanguloEnSensorRespetaElDesplazamientoDelCropDelViewport() {
        val sinDesplazar = rectanguloEnSensor(RectanguloEntero(0, 0, 1600, 1080), 90, RegionGuiaOcr.TARJETA_ID)
        val desplazado = rectanguloEnSensor(RectanguloEntero(160, 0, 1760, 1080), 90, RegionGuiaOcr.TARJETA_ID)
        assertEquals(sinDesplazar.left + 160, desplazado.left)
        assertEquals(sinDesplazar.top, desplazado.top)
    }

    // --- recortarYuvANv21 ---

    @Test
    fun recortarYuvANv21CopiaSoloLaRegionDelPlanoYRespetandoElRelleno() {
        // Plano Y de 6x4 con rowStride 8 (2 bytes de relleno por fila);
        // cada byte = fila*10 + columna para reconocer de dónde vino.
        val (ancho, alto, stride) = Triple(6, 4, 8)
        val y = ByteArray(stride * alto) { i -> if (i % stride < ancho) ((i / stride) * 10 + i % stride).toByte() else 99 }
        val u = ByteArray(8) { 50 }
        val v = ByteArray(8) { 60 }
        val rect = RectanguloEntero(left = 2, top = 2, right = 6, bottom = 4)

        val nv21 = recortarYuvANv21(rect, y, stride, u, v, uvRowStride = 4, uvPixelStride = 1)

        assertEquals(4 * 2 + 4, nv21.size)
        assertEquals(listOf(22, 23, 24, 25, 32, 33, 34, 35), nv21.take(8).map { it.toInt() })
    }

    @Test
    fun recortarYuvANv21IntercalaVURespetandoPixelStride() {
        // Croma de 4x2 (imagen 8x4) con pixelStride 2: las muestras válidas
        // están en índices pares; en los impares hay basura.
        val y = ByteArray(8 * 4)
        val u = ByteArray(16) { i -> if (i % 2 == 0) (100 + i / 2).toByte() else 0 }
        val v = ByteArray(16) { i -> if (i % 2 == 0) (200 + i / 2).toByte() else 0 }
        val rect = RectanguloEntero(left = 4, top = 2, right = 8, bottom = 4) // 4x2 -> croma 2x1

        val nv21 = recortarYuvANv21(rect, y, 8, u, v, uvRowStride = 8, uvPixelStride = 2)

        // Croma: fila 1 (top/2), columnas 2 y 3 (left/2) -> índices 8+4, 8+6.
        val croma = nv21.drop(4 * 2).map { it.toInt() and 0xff }
        assertEquals(listOf(200 + 6, 100 + 6, 200 + 7, 100 + 7), croma)
    }
}
