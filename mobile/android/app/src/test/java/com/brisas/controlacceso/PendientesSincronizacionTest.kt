package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PendientesSincronizacionTest {

    @Test
    fun avisosConTablaPidenSoloEsasTablasSinRepetir() {
        val pendientes = PendientesSincronizacion()
        pendientes.anotar("ingresos")
        pendientes.anotar("gafetes")
        pendientes.anotar("ingresos")
        assertEquals(listOf("ingresos", "gafetes"), pendientes.tomar())
    }

    @Test
    fun unPedidoCompletoGanaSobreLasTablas() {
        val pendientes = PendientesSincronizacion()
        pendientes.anotar("empresas")
        pendientes.anotar(null) // registro local o reconexión del canal
        assertNull(pendientes.tomar())
    }

    @Test
    fun sinNadaAnotadoPideLaCompleta() {
        assertNull(PendientesSincronizacion().tomar())
    }

    @Test
    fun tomarDejaVacioParaLaSiguienteCorrida() {
        val pendientes = PendientesSincronizacion()
        pendientes.anotar("citas")
        pendientes.tomar()
        pendientes.anotar("usuarios")
        assertEquals(listOf("usuarios"), pendientes.tomar())
    }
}
