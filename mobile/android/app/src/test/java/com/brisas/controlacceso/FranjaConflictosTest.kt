package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test

// Los textos de cada conflicto los redacta y los prueba el núcleo
// (`nube::mensajes_conflictos`); acá sólo queda el título de la franja.
class FranjaConflictosTest {
    @Test
    fun `el titulo cuenta los conflictos en singular y plural`() {
        assertEquals("1 conflicto por resolver", tituloConflictos(1))
        assertEquals("3 conflictos por resolver", tituloConflictos(3))
    }
}
