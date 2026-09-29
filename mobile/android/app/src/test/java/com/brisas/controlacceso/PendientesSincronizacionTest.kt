package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test

class PendientesSincronizacionTest {

    @Test
    fun avisosConTablaPidenSoloEsasTablasSinRepetir() {
        val pendientes = PendientesSincronizacion()
        pendientes.anotar("ingresos")
        pendientes.anotar("gafetes")
        pendientes.anotar("ingresos")
        assertEquals(AlcancePendiente.Tablas(listOf("ingresos", "gafetes")), pendientes.tomar())
    }

    @Test
    fun unPedidoCompletoGanaSobreLasTablas() {
        val pendientes = PendientesSincronizacion()
        pendientes.anotar("empresas")
        pendientes.anotar(null) // reconexión del canal
        assertEquals(AlcancePendiente.Completa, pendientes.tomar())
    }

    @Test
    fun sinNadaAnotadoPideLaCompleta() {
        assertEquals(AlcancePendiente.Completa, PendientesSincronizacion().tomar())
    }

    @Test
    fun tomarDejaVacioParaLaSiguienteCorrida() {
        val pendientes = PendientesSincronizacion()
        pendientes.anotar("citas")
        pendientes.tomar()
        pendientes.anotar("usuarios")
        assertEquals(AlcancePendiente.Tablas(listOf("usuarios")), pendientes.tomar())
    }

    @Test
    fun unCambioLocalSoloPideElEnvio() {
        val pendientes = PendientesSincronizacion()
        pendientes.anotarCambioLocal()
        pendientes.anotarCambioLocal()
        assertEquals(AlcancePendiente.SoloEnvio, pendientes.tomar())
    }

    @Test
    fun lasTablasYaSubenLoLocal() {
        val pendientes = PendientesSincronizacion()
        pendientes.anotarCambioLocal()
        pendientes.anotar("ingresos")
        assertEquals(AlcancePendiente.Tablas(listOf("ingresos")), pendientes.tomar())
    }

    @Test
    fun laCompletaGanaSobreUnCambioLocal() {
        val pendientes = PendientesSincronizacion()
        pendientes.anotarCambioLocal()
        pendientes.anotar(null)
        assertEquals(AlcancePendiente.Completa, pendientes.tomar())
    }

    @Test
    fun elEnvioNoQuedaPendienteParaLaSiguienteCorrida() {
        val pendientes = PendientesSincronizacion()
        pendientes.anotarCambioLocal()
        pendientes.tomar()
        // Sin nada nuevo: el pulso periódico, que es completo.
        assertEquals(AlcancePendiente.Completa, pendientes.tomar())
    }
}
