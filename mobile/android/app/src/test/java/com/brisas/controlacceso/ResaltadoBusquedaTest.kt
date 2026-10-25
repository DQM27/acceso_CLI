package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ResaltadoBusquedaTest {
    @Test
    fun `sin consulta no resalta nada`() {
        assertTrue(rangosCoincidencia("Ana Mora", "").isEmpty())
        assertTrue(rangosCoincidencia("Ana Mora", "   ").isEmpty())
        assertTrue(rangosCoincidencia("", "ana").isEmpty())
    }

    @Test
    fun `no distingue mayusculas ni tildes`() {
        assertEquals(listOf(0..3), rangosCoincidencia("José Pérez", "jose"))
        assertEquals(listOf(5..9), rangosCoincidencia("José Pérez", "PEREZ"))
        // Y al revés: la tilde en la consulta encuentra el texto sin ella.
        assertEquals(listOf(0..3), rangosCoincidencia("JOSE PEREZ", "josé"))
    }

    @Test
    fun `cada palabra se busca por separado y en cualquier orden`() {
        assertEquals(listOf(0..2, 4..7), rangosCoincidencia("Ana Mora", "mora ana"))
    }

    @Test
    fun `encuentra todas las apariciones`() {
        assertEquals(listOf(0..0, 3..3), rangosCoincidencia("1-21", "1"))
    }

    @Test
    fun `los rangos que se tocan o se pisan quedan en uno`() {
        assertEquals(listOf(0..5), rangosCoincidencia("Mariana", "mari ian"))
        assertEquals(listOf(0..5), rangosCoincidencia("abcdef", "abc def"))
    }

    @Test
    fun `las comas separan como los espacios`() {
        assertEquals(listOf(7..8), rangosCoincidencia("Gafete 12", "12,"))
    }

    @Test
    fun `sirve para filtrar por nombre o codigo`() {
        assertTrue(rangosCoincidencia("Ramírez", "ramirez").isNotEmpty())
        assertTrue(rangosCoincidencia("EMP-0042", "0042").isNotEmpty())
        assertTrue(rangosCoincidencia("Otro", "ramirez").isEmpty())
    }
}
