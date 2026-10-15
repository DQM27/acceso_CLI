package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class IdentidadEquipoTest {
    @Test
    fun laUnidadOperativaVaSolaYEnMayusculas() {
        assertEquals("BRISAS", textoUnidadOperativa("Brisas"))
        assertEquals("BELÉN CARTAGO", textoUnidadOperativa(" Belén Cartago "))
    }

    @Test
    fun sinUnidadNoMuestraNada() {
        assertNull(textoUnidadOperativa(null))
        assertNull(textoUnidadOperativa("  "))
    }
}
