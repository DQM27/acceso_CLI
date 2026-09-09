package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class TiempoTest {
    @Test
    fun `acepta z y offset numerico`() {
        assertNotNull(instanteFechaHora("2026-09-09T12:00:00Z"))
        assertNotNull(instanteFechaHora("2026-09-09T06:00:00-06:00"))
    }

    @Test
    fun `fecha remota malformada no derriba la pantalla`() {
        assertNull(instanteFechaHora("fecha-invalida"))
        assertEquals("Fecha no disponible", textoFechaHora("fecha-invalida"))
    }
}
