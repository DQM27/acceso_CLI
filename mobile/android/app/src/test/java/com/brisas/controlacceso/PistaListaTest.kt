package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test

class PistaListaTest {

    @Test
    fun laPistaDeAdentroDiceCuantosYQueHacer() {
        assertEquals("12 adentro · Toque a alguien para registrar su salida", pistaListaAdentro(12))
    }

    @Test
    fun laPistaDeKofConcuerdaEnNumero() {
        assertEquals("1 prestado · Toque uno para registrar su devolución", pistaListaPrestados(1))
        assertEquals("3 prestados · Toque uno para registrar su devolución", pistaListaPrestados(3))
    }
}
