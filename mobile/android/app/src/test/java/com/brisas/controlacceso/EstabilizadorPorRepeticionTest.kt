package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class EstabilizadorPorRepeticionTest {

    private fun porRepeticion() = EstabilizadorPorRepeticion(extraer = ::extraerVehiculo, clave = { it.claveVotacion() })

    private fun porVotacion() = EstabilizadorPorRepeticion(
        extraer = ::extraerVehiculo,
        clave = { it.claveVotacion() },
        desdeClave = ::vehiculoDesdeClave,
    )

    @Test
    fun sinVotacionExigeLaMismaLecturaEntera() {
        val estabilizador = porRepeticion()
        assertNull(estabilizador.procesarFrame("BPH485"))
        assertEquals("BPH485", estabilizador.procesarFrame("BPH485")?.valor)
    }

    @Test
    fun conVotacionCombinaFramesQueErranEnCaracteresDistintos() {
        // Cada frame lee mal un dígito distinto: ninguno coincide entero
        // con otro, pero cada posición sí es mayoría.
        val estabilizador = porVotacion()
        assertNull(estabilizador.procesarFrame("BPH435"))
        assertNull(estabilizador.procesarFrame("BPH486"))
        val placa = estabilizador.procesarFrame("BPH485")
        assertEquals("BPH485", placa?.valor)
        assertEquals(TipoVehiculoDetectado.PLACA, placa?.tipo)
    }

    @Test
    fun conVotacionOtroVehiculoEmpiezaDeCero() {
        val estabilizador = porVotacion()
        estabilizador.procesarFrame("BPH485")
        // Otra placa completa: no se mezcla con la anterior.
        assertNull(estabilizador.procesarFrame("KLM902"))
        assertEquals("KLM902", estabilizador.procesarFrame("KLM902")?.valor)
    }

    @Test
    fun claveDeVehiculoIdaYVuelta() {
        val vehiculo = VehiculoRutaDetectado("22906", TipoVehiculoDetectado.NUMERO_UNIDAD)
        assertEquals(vehiculo, vehiculoDesdeClave(vehiculo.claveVotacion()))
        assertNull(vehiculoDesdeClave("DESCONOCIDO:123"))
        assertNull(vehiculoDesdeClave("sin separador"))
    }
}
