package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test

class InterfazPrincipalTest {
    @Test
    fun `la confirmacion de salida solo acompana el nombre con el gafete`() {
        assertEquals("Gafete 25", textoGafeteSalida(25))
        assertEquals("Sin gafete", textoGafeteSalida(null))
    }

    @Test
    fun `cada toque de la interfaz usa un tipo distinto del sistema`() {
        val tipos = Haptico.entries.map(::tipoHaptico)
        assertEquals(tipos.size, tipos.toSet().size)
    }
}
