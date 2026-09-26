package com.brisas.controlacceso

import org.junit.Assert.assertEquals
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

    // --- construirNv21 ---

    @Test
    fun construirNv21CopiaElPlanoYRespetandoElRelleno() {
        // Imagen 2x2 con rowStride de 3 (un byte de relleno al final de
        // cada fila) -- si el recorte no respetara el stride, el segundo
        // byte de la fila 2 arrastraría el relleno de la fila 1.
        val y = byteArrayOf(1, 2, 9, 3, 4, 9) // fila0: [1,2,relleno] fila1: [3,4,relleno]
        val u = byteArrayOf(5)
        val v = byteArrayOf(6)

        val nv21 = construirNv21(
            ancho = 2, alto = 2,
            y = y, yRowStride = 3,
            u = u, v = v, uvRowStride = 1, uvPixelStride = 1,
        )

        // Plano Y sin relleno: 1,2,3,4 -- después VU intercalado: v,u = 6,5.
        assertEquals(listOf<Byte>(1, 2, 3, 4, 6, 5), nv21.toList())
    }

    @Test
    fun construirNv21IntercalaVURespetandoPixelStride() {
        // Croma con pixelStride 2 (por ejemplo, un plano semi-planar donde U
        // y V comparten buffer intercalado y sólo se toma cada 2 bytes).
        val y = byteArrayOf(0, 0, 0, 0) // 2x2, sin relleno (rowStride=ancho)
        val u = byteArrayOf(10, 99, 11, 99) // valores reales en índices 0 y 2
        val v = byteArrayOf(20, 99, 21, 99)

        val nv21 = construirNv21(
            ancho = 2, alto = 2,
            y = y, yRowStride = 2,
            u = u, v = v, uvRowStride = 2, uvPixelStride = 2,
        )

        // Croma 1x1 (ancho/2 x alto/2 = 1x1) -- sólo el primer par V,U.
        assertEquals(listOf<Byte>(0, 0, 0, 0, 20, 10), nv21.toList())
    }

    // --- convertirNv21AArgb ---

    @Test
    fun convertirNv21AArgbDaBlancoParaYMaximoYCromaNeutro() {
        // Y=255 (blanco), croma neutro (U=V=128, sin color) -- imagen 2x2,
        // un solo par de croma para los 4 píxeles.
        val nv21 = byteArrayOf(255.toByte(), 255.toByte(), 255.toByte(), 255.toByte(), 128.toByte(), 128.toByte())

        val pixeles = convertirNv21AArgb(nv21, ancho = 2, alto = 2)

        // 0xFFFFFFFF (alpha 255, R=G=B=255) como Int con signo = -1.
        assertEquals(listOf(-1, -1, -1, -1), pixeles.toList())
    }

    @Test
    fun convertirNv21AArgbDaNegroParaYMinimoYCromaNeutro() {
        val nv21 = byteArrayOf(0, 0, 0, 0, 128.toByte(), 128.toByte())

        val pixeles = convertirNv21AArgb(nv21, ancho = 2, alto = 2)

        // 0xFF000000 (alpha 255, R=G=B=0) como Int con signo = -16777216.
        assertEquals(listOf(-16777216, -16777216, -16777216, -16777216), pixeles.toList())
    }
}
