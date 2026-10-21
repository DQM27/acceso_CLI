package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class VibracionEscaneoTest {

    private fun golpes(patron: PatronVibracion) = patron.amplitudes.count { it > 0 }

    private fun duracion(patron: PatronVibracion) = patron.tiempos.sum()

    @Test
    fun cadaPatronTieneUnaAmplitudPorTramo() {
        for (patron in listOf(PATRON_ERROR, PATRON_ADVERTENCIA, PATRON_NO_ENCONTRADO)) {
            assertEquals(patron.tiempos.size, patron.amplitudes.size)
            // Empieza sin pausa y las pausas no vibran.
            assertEquals(0L, patron.tiempos.first())
            assertEquals(0, patron.amplitudes.first())
        }
    }

    @Test
    fun laAdvertenciaSeDistingueDelErrorPorCantidadYDuracion() {
        assertEquals(2, golpes(PATRON_ERROR))
        assertEquals(3, golpes(PATRON_ADVERTENCIA))
        assertNotEquals(duracion(PATRON_ERROR), duracion(PATRON_ADVERTENCIA))
        assertTrue(duracion(PATRON_ADVERTENCIA) > duracion(PATRON_ERROR))
    }

    @Test
    fun noEncontradoEsUnSoloGolpe() {
        // Distinto del doble golpe de error y del triple de advertencia.
        assertEquals(1, golpes(PATRON_NO_ENCONTRADO))
    }

    @Test
    fun unPatronConTramosDesparejosSeRechaza() {
        val rechazado = runCatching { PatronVibracion(longArrayOf(0, 50), intArrayOf(0)) }
        assertTrue(rechazado.isFailure)
    }
}
