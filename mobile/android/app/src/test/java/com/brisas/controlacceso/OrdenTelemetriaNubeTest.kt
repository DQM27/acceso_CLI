package com.brisas.controlacceso

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class OrdenTelemetriaNubeTest {
    @Test
    fun `acepta la orden de este telefono`() {
        assertTrue(OrdenTelemetriaNube.esParaEsteEquipo("d1", dispositivoId = "d1"))
    }

    @Test
    fun `descarta la orden de otro equipo del sitio`() {
        assertFalse(OrdenTelemetriaNube.esParaEsteEquipo("d2", dispositivoId = "d1"))
    }

    @Test
    fun `descarta avisos sin dispositivo`() {
        assertFalse(OrdenTelemetriaNube.esParaEsteEquipo(null, dispositivoId = "d1"))
        assertFalse(OrdenTelemetriaNube.esParaEsteEquipo("", dispositivoId = "d1"))
    }
}
