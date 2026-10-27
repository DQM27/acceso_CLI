package com.brisas.controlacceso

import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.text.input.KeyboardType
import org.junit.Assert.assertEquals
import org.junit.Test

class InterfazPrincipalTest {
    @Test
    fun `la confirmacion de salida solo acompana el nombre con el gafete`() {
        assertEquals("Gafete 25", textoGafeteSalida(25))
        assertEquals("Sin gafete", textoGafeteSalida(null))
    }

    @Test
    fun `el buscador de Activos cambia de ayuda y teclado con el modo`() {
        assertEquals("Nombre o cédula", ayudaBusquedaActivos(ModoBusqueda.ENTRADA))
        assertEquals("Gafetes: 2, 25, 85", ayudaBusquedaActivos(ModoBusqueda.SALIDA_GAFETE))
        assertEquals(KeyboardType.Number, tecladoBusquedaActivos(ModoBusqueda.SALIDA_GAFETE).keyboardType)
        assertEquals(KeyboardOptions.Default, tecladoBusquedaActivos(ModoBusqueda.ENTRADA))
    }

    @Test
    fun `cada toque de la interfaz usa un tipo distinto del sistema`() {
        val tipos = Haptico.entries.map(::tipoHaptico)
        assertEquals(tipos.size, tipos.toSet().size)
    }
}
