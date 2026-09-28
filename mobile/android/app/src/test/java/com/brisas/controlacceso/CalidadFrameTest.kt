package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CalidadFrameTest {

    /// Tablero de bloques de 8 px oscuros/claros: bordes nítidos en las dos
    /// direcciones, como el texto de un documento. (El estimador toma el
    /// mínimo entre direcciones, así que rayas en un solo sentido darían 0.)
    private fun tablero(ancho: Int, alto: Int) = ByteArray(ancho * alto) { i ->
        val (x, y) = i % ancho to i / ancho
        (if ((x / 8 + y / 8) % 2 == 0) 20 else 200).toByte()
    }

    /// El mismo tablero con transiciones suaves (desenfocado).
    private fun tableroBorroso(ancho: Int, alto: Int) = ByteArray(ancho * alto) { i ->
        val (x, y) = i % ancho to i / ancho
        val onda = { v: Int -> 0.5 - 0.5 * kotlin.math.cos((v % 16) / 16.0 * 2 * Math.PI) }
        (20 + 180 * (onda(x) + onda(y)) / 2).toInt().toByte()
    }

    @Test
    fun imagenNitidaMideMasQueLaMismaImagenBorrosa() {
        val nitida = medirCalidad(tablero(64, 48), 64, 48)
        val borrosa = medirCalidad(tableroBorroso(64, 48), 64, 48)
        assertTrue("${nitida.nitidez} vs ${borrosa.nitidez}", nitida.nitidez > borrosa.nitidez)
    }

    @Test
    fun imagenPlanaNoTieneNitidezNiReflejo() {
        val calidad = medirCalidad(ByteArray(32 * 32) { 100 }, 32, 32)
        assertEquals(0f, calidad.nitidez)
        assertEquals(0f, calidad.fraccionReflejo)
    }

    @Test
    fun cuentaLosPixelesSaturadosComoReflejo() {
        // Mitad superior saturada.
        val y = ByteArray(32 * 32) { i -> if (i < 32 * 16) 255.toByte() else 90 }
        val calidad = medirCalidad(y, 32, 32, paso = 1)
        assertTrue("${calidad.fraccionReflejo}", calidad.fraccionReflejo in 0.45f..0.55f)
    }

    @Test
    fun elPrimerFrameSiempreSeProcesaConPesoCompleto() {
        val decision = FiltroCalidad().evaluar(CalidadFrame(nitidez = 10f, fraccionReflejo = 0f))
        assertTrue(decision.procesar)
        assertEquals(1f, decision.peso)
    }

    @Test
    fun descartaUnFrameMuchoMasBorrosoQueLosRecientes() {
        val filtro = FiltroCalidad(fraccionMinima = 0.6f)
        filtro.evaluar(CalidadFrame(40f, 0f))
        val movido = filtro.evaluar(CalidadFrame(10f, 0f))
        assertFalse(movido.procesar)
        val casiIgual = filtro.evaluar(CalidadFrame(36f, 0f))
        assertTrue(casiIgual.procesar)
        assertEquals(0.9f, casiIgual.peso, 1e-6f)
    }

    @Test
    fun nuncaDescartaMasDeLoPermitidoSeguido() {
        // Si la cámara no consigue nada mejor, se lee lo que hay: un umbral
        // relativo no puede dejar al escáner sin leer para siempre.
        val filtro = FiltroCalidad(maximoDescartesSeguidos = 2)
        filtro.evaluar(CalidadFrame(50f, 0f))
        assertFalse(filtro.evaluar(CalidadFrame(5f, 0f)).procesar)
        assertFalse(filtro.evaluar(CalidadFrame(5f, 0f)).procesar)
        val forzado = filtro.evaluar(CalidadFrame(5f, 0f))
        assertTrue(forzado.procesar)
        assertEquals(0.2f, forzado.peso, 1e-6f)
    }

    @Test
    fun sinBordesSignificativosNoSeDescartaNada() {
        // Cámara tapada o documento fuera de cuadro: 2 contra 6 es ruido.
        val filtro = FiltroCalidad()
        filtro.evaluar(CalidadFrame(6f, 0f))
        val decision = filtro.evaluar(CalidadFrame(2f, 0f))
        assertTrue(decision.procesar)
        assertEquals(1f, decision.peso)
    }

    @Test
    fun laVentanaOlvidaUnFrameNitidoViejo() {
        val filtro = FiltroCalidad(ventana = 2)
        filtro.evaluar(CalidadFrame(100f, 0f))
        filtro.evaluar(CalidadFrame(20f, 0f))
        // El 100 ya salió de la ventana: 20 es la referencia.
        assertTrue(filtro.evaluar(CalidadFrame(19f, 0f)).procesar)
    }
}
