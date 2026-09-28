package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.TipoIngreso

class CoincidenciaEscaneoTest {

    private fun contratista(id: Long, cedula: String, nombre: String) = ContratistaResumen(
        id = id,
        cedula = cedula,
        nombre = nombre,
        empresaNombre = "Empresa Test",
        tipoIngreso = TipoIngreso.PRAIND,
        fechaVencimientoPraind = null,
        tieneAcceso = true,
        tieneIngresoActivo = false,
        avisoAcceso = null,
    )

    @Test
    fun eligeElUnicoResultadoCuyaCedulaEsExactamenteLaEscaneada() {
        val resultados = listOf(contratista(1, "1-1234-0567", "ANA"), contratista(2, "112340568", "LUIS"))
        assertEquals(1L, contratistaEscaneadoClaro("112340567", resultados)?.id)
    }

    @Test
    fun unUnicoResultadoConOtraCedulaNoSeEligeSolo() {
        // Un dígito mal leído que la búsqueda parcial igual encuentra: antes
        // se abría el formulario de esa persona como si fuera la escaneada.
        val resultados = listOf(contratista(1, "112340568", "LUIS"))
        assertNull(contratistaEscaneadoClaro("112340567", resultados))
    }

    @Test
    fun porNombreSinDigitosSeMantieneElResultadoUnico() {
        val resultados = listOf(contratista(1, "112340567", "JUAN PEREZ MORA"))
        assertEquals(1L, contratistaEscaneadoClaro("JUAN PEREZ MORA", resultados)?.id)
    }

    @Test
    fun porNombreConVariosResultadosNoSeEligeNinguno() {
        val resultados = listOf(contratista(1, "112340567", "JUAN PEREZ"), contratista(2, "212340567", "JUAN PEREZ"))
        assertNull(contratistaEscaneadoClaro("JUAN PEREZ", resultados))
    }
}
