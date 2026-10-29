package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import uniffi.control_acceso_mobile.AvisoAcceso
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.TipoIngreso

class MarcaContratistaTest {

    private fun contratista(adentro: Boolean, aviso: AvisoAcceso?) = ContratistaResumen(
        id = 1,
        cedula = "117900735",
        nombre = "BERTA",
        empresaNombre = "BAC",
        tipoIngreso = TipoIngreso.PRAIND,
        fechaVencimientoPraind = null,
        tieneAcceso = aviso != AvisoAcceso.ACCESO_DENEGADO,
        tieneIngresoActivo = adentro,
        avisoAcceso = null,
        tipoAvisoAcceso = aviso,
    )

    @Test
    fun sinNadaQueAvisarLaTarjetaVaNormal() {
        assertNull(marcaContratista(contratista(adentro = false, aviso = null)))
    }

    @Test
    fun cadaAvisoTieneSuMarca() {
        assertEquals(MarcaContratista.ADENTRO, marcaContratista(contratista(adentro = true, aviso = null)))
        assertEquals(MarcaContratista.DENEGADO, marcaContratista(contratista(adentro = false, aviso = AvisoAcceso.ACCESO_DENEGADO)))
        assertEquals(MarcaContratista.VENCIDO, marcaContratista(contratista(adentro = false, aviso = AvisoAcceso.PRAIND_VENCIDO)))
    }

    @Test
    fun adentroGanaAunqueNoPuedaVolverAEntrar() {
        // Con esa persona lo que se hace es la salida, que siempre se permite.
        assertEquals(MarcaContratista.ADENTRO, marcaContratista(contratista(adentro = true, aviso = AvisoAcceso.ACCESO_DENEGADO)))
        assertEquals(MarcaContratista.ADENTRO, marcaContratista(contratista(adentro = true, aviso = AvisoAcceso.PRAIND_VENCIDO)))
    }

    @Test
    fun lasPalabrasSonCortas() {
        assertEquals(listOf("ADENTRO", "DENEGADO", "VENCIDO"), MarcaContratista.entries.map { it.palabra })
    }
}
