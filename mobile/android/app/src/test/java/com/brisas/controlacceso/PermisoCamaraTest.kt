package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test

class PermisoCamaraTest {
    @Test
    fun `con permiso se abre la camara`() {
        assertEquals(
            PasoPermisoCamara.CONCEDIDO,
            pasoPermisoCamara(concedido = true, negadoUnaVez = true, mostrarExplicacion = false),
        )
    }

    @Test
    fun `antes de la primera negativa se vuelve a pedir`() {
        assertEquals(
            PasoPermisoCamara.PEDIR,
            pasoPermisoCamara(concedido = false, negadoUnaVez = false, mostrarExplicacion = false),
        )
    }

    @Test
    fun `negado una vez con explicacion se puede volver a pedir`() {
        assertEquals(
            PasoPermisoCamara.PEDIR,
            pasoPermisoCamara(concedido = false, negadoUnaVez = true, mostrarExplicacion = true),
        )
    }

    /// Negado "para siempre": el diálogo ya no aparece, sólo queda Ajustes.
    @Test
    fun `negado sin explicacion manda a Ajustes`() {
        assertEquals(
            PasoPermisoCamara.IR_A_AJUSTES,
            pasoPermisoCamara(concedido = false, negadoUnaVez = true, mostrarExplicacion = false),
        )
    }
}
