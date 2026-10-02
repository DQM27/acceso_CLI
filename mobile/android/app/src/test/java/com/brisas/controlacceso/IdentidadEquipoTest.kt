package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class IdentidadEquipoTest {
    @Test
    fun muestraUnidadYEtiqueta() {
        assertEquals(
            "Unidad: Planta Cartago · Celular portería norte",
            textoIdentidadEquipo("Planta Cartago", "Celular portería norte"),
        )
    }

    @Test
    fun muestraLoQueHayaSiFaltaUnaParte() {
        assertEquals("Unidad: Planta Cartago", textoIdentidadEquipo("Planta Cartago", null))
        assertEquals("Equipo: Celular portería norte", textoIdentidadEquipo(null, "Celular portería norte"))
    }

    @Test
    fun noMuestraNadaSiTodaviaNoLlego() {
        assertNull(textoIdentidadEquipo(null, null))
        assertNull(textoIdentidadEquipo("  ", ""))
    }
}
