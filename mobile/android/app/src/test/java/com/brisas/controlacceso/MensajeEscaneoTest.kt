package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test

class MensajeEscaneoTest {
    @Test
    fun `cada estado de la lectura tiene su tono`() {
        assertEquals(TonoEscaneo.GUIA, tonoEscaneo(EstadoEscaneo.BUSCANDO))
        assertEquals(TonoEscaneo.EXITO, tonoEscaneo(EstadoEscaneo.CONFIRMADO))
        assertEquals(TonoEscaneo.ERROR, tonoEscaneo(EstadoEscaneo.INVALIDO))
    }

    @Test
    fun `leido pero vencido es una advertencia`() {
        assertEquals(TonoEscaneo.ADVERTENCIA, tonoEscaneo(EstadoEscaneo.CONFIRMADO, vencido = true))
    }

    @Test
    fun `el resultado real del escaneo continuo manda sobre la lectura`() {
        // Gafete leído bien pero sin ingreso activo: la salida falló.
        assertEquals(TonoEscaneo.ERROR, tonoEscaneo(EstadoEscaneo.CONFIRMADO, resultadoEsError = true))
        assertEquals(TonoEscaneo.EXITO, tonoEscaneo(EstadoEscaneo.CONFIRMADO, resultadoEsError = false))
    }
}
