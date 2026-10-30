package com.brisas.controlacceso

import java.time.LocalDate
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class NieveNavidenaTest {

    @Test
    fun `la temporada va del 1 de diciembre al 6 de enero`() {
        assertFalse(esTemporadaNavidena(LocalDate.of(2026, 11, 30)))
        assertTrue(esTemporadaNavidena(LocalDate.of(2026, 12, 1)))
        assertTrue(esTemporadaNavidena(LocalDate.of(2026, 12, 31)))
        assertTrue(esTemporadaNavidena(LocalDate.of(2027, 1, 6)))
        assertFalse(esTemporadaNavidena(LocalDate.of(2027, 1, 7)))
        assertFalse(esTemporadaNavidena(LocalDate.of(2026, 10, 29)))
    }

    @Test
    fun `los copos se ven enteros en la barra y se apagan sobre la tarjeta`() {
        val finBarra = 100f
        val desborde = 40f
        assertEquals(0f, desvanecer(-1f, finBarra, desborde), 0f)
        assertEquals(0.5f, desvanecer(6f, finBarra, desborde), 0.001f)
        assertEquals(1f, desvanecer(50f, finBarra, desborde), 0f)
        assertEquals(1f, desvanecer(100f, finBarra, desborde), 0f)
        assertEquals(0.5f, desvanecer(120f, finBarra, desborde), 0.001f)
        assertEquals(0f, desvanecer(140f, finBarra, desborde), 0f)
        assertEquals(0f, desvanecer(200f, finBarra, desborde), 0f)
    }
}
