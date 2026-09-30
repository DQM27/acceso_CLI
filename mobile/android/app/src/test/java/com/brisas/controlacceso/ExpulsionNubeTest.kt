package com.brisas.controlacceso

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ExpulsionNubeTest {
    @Test
    fun `solo es para el dispositivo que nombra`() {
        assertTrue(ExpulsionNube.esParaEsteEquipo("d1", dispositivoId = "d1"))
        assertFalse(ExpulsionNube.esParaEsteEquipo("d2", dispositivoId = "d1"))
    }

    @Test
    fun `descarta avisos sin dispositivo`() {
        assertFalse(ExpulsionNube.esParaEsteEquipo(null, dispositivoId = "d1"))
    }
}
