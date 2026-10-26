package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ExpulsionNubeTest {
    @Test
    fun `solo es para el dispositivo que nombra`() {
        assertTrue(ExpulsionNube.esParaEsteEquipo("d1", dispositivoId = "d1"))
        assertFalse(ExpulsionNube.esParaEsteEquipo("d2", dispositivoId = "d1"))
    }

    @Test
    fun `el cierre de sesion solo es para el usuario de este telefono`() {
        assertTrue(ExpulsionNube.esCierreDeEstaSesion("900000301", cedula = "900000301"))
        assertFalse(ExpulsionNube.esCierreDeEstaSesion("900000302", cedula = "900000301"))
        assertFalse(ExpulsionNube.esCierreDeEstaSesion(null, cedula = "900000301"))
        assertFalse(ExpulsionNube.esCierreDeEstaSesion("", cedula = ""))
    }

    @Test
    fun `descarta avisos sin dispositivo`() {
        assertFalse(ExpulsionNube.esParaEsteEquipo(null, dispositivoId = "d1"))
    }

    // Incidente del 2026-10-26: el teléfono se mostró como retirado sin
    // estarlo. El aviso en vivo sólo dispara la pregunta a la nube; lo que
    // manda es lo que el núcleo anotó con la respuesta de `device-auth`.

    @Test
    fun `el aviso solo se confirma si la nube dijo retirado`() {
        var preguntas = 0
        assertTrue(ExpulsionNube.confirmadoPorLaNube({ preguntas++ }, retirado = { true }))
        assertFalse(ExpulsionNube.confirmadoPorLaNube({ preguntas++ }, retirado = { false }))
        assertEquals("siempre pregunta antes de decidir", 2, preguntas)
    }

    @Test
    fun `si no se pudo preguntar a la nube no esta confirmado`() {
        var consultoLaMarca = false
        val confirmado = ExpulsionNube.confirmadoPorLaNube(
            preguntarALaNube = { throw IllegalStateException("sin red") },
            retirado = {
                consultoLaMarca = true
                true
            },
        )
        assertFalse(confirmado)
        assertFalse("sin respuesta de la nube no hay nada que leer", consultoLaMarca)
    }

    @Test
    fun `si no se puede leer la marca no esta confirmado`() {
        assertFalse(ExpulsionNube.confirmadoPorLaNube({}, retirado = { throw IllegalStateException("base") }))
    }
}
