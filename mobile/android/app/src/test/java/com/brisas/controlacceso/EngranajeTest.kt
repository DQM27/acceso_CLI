package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test

class EngranajeTest {
    @Test
    fun `el trazo es identico al del escritorio`() {
        // Salida de `trazoEngranaje(8, 47, 37)` de
        // desktop/src/componentes/Engranaje.logica.ts, copiada tal cual.
        val escritorio = "M42.21 13.83 L43.74 3.42 L56.26 3.42 L57.79 13.83 A37 37 0 0 1 70.07 18.92 " +
            "L78.51 12.64 L87.36 21.49 L81.08 29.93 A37 37 0 0 1 86.17 42.21 L96.58 43.74 L96.58 56.26 " +
            "L86.17 57.79 A37 37 0 0 1 81.08 70.07 L87.36 78.51 L78.51 87.36 L70.07 81.08 " +
            "A37 37 0 0 1 57.79 86.17 L56.26 96.58 L43.74 96.58 L42.21 86.17 A37 37 0 0 1 29.93 81.08 " +
            "L21.49 87.36 L12.64 78.51 L18.92 70.07 A37 37 0 0 1 13.83 57.79 L3.42 56.26 L3.42 43.74 " +
            "L13.83 42.21 A37 37 0 0 1 18.92 29.93 L12.64 21.49 L21.49 12.64 L29.93 18.92 " +
            "A37 37 0 0 1 42.21 13.83 Z"

        assertEquals(escritorio, trazoEngranaje(8, 47, 37))
    }

    @Test
    fun `un arco une cada diente con el siguiente`() {
        val trazo = trazoEngranaje(6, 47, 37)

        assertEquals(6, trazo.count { it == 'A' })
        assertEquals(6 * 3, trazo.count { it == 'L' })
    }
}
