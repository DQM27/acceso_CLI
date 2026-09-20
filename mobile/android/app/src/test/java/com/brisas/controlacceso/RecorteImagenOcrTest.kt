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
}
