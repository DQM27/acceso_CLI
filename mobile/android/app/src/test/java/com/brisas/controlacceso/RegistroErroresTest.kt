package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test

class RegistroErroresTest {
    @Test
    fun `el titulo lleva la clase y el mensaje`() {
        assertEquals(
            "java.lang.IllegalStateException: sin cámara",
            tituloDeExcepcion(IllegalStateException("sin cámara")),
        )
    }

    @Test
    fun `sin mensaje queda solo la clase`() {
        assertEquals("java.lang.NullPointerException", tituloDeExcepcion(NullPointerException()))
        assertEquals("java.lang.RuntimeException", tituloDeExcepcion(RuntimeException("  ")))
    }

    @Test
    fun `una excepcion que envuelve a otra nombra la causa de fondo`() {
        val error = RuntimeException("falló el ingreso", IllegalArgumentException("x", java.io.IOException("disco lleno")))
        assertEquals(
            "java.lang.RuntimeException: falló el ingreso (causa: java.io.IOException: disco lleno)",
            tituloDeExcepcion(error),
        )
    }

    @Test
    fun `el titulo de una salida anterior lleva la descripcion si hay`() {
        assertEquals("ANR: Input dispatching timed out", tituloDeSalida("ANR", "Input dispatching timed out"))
        assertEquals("Fallo nativo", tituloDeSalida("Fallo nativo", null))
        assertEquals("Fallo nativo", tituloDeSalida("Fallo nativo", ""))
    }

    @Test
    fun `sin instalar no llama al nucleo`() {
        // En la JVM de las pruebas no hay librería nativa: si llamara, fallaría.
        RegistroErrores.operacion("buscarContratistas", true)
        RegistroErrores.pantalla("Ingreso")
        RegistroErrores.enviarYa()
    }
}
