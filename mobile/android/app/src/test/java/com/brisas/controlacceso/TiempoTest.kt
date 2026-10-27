package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test
import java.time.LocalDate
import java.time.ZoneId

class TiempoTest {
    @Test
    fun `acepta z y offset numerico`() {
        assertNotNull(instanteFechaHora("2026-09-09T12:00:00Z"))
        assertNotNull(instanteFechaHora("2026-09-09T06:00:00-06:00"))
    }

    @Test
    fun `fecha remota malformada no derriba la pantalla`() {
        assertNull(instanteFechaHora("fecha-invalida"))
        assertEquals("Fecha no disponible", textoFechaHora("fecha-invalida"))
    }

    private val costaRica = ZoneId.of("America/Costa_Rica")

    @Test
    fun `un ingreso de hoy muestra solo la hora local`() {
        // 13:44 UTC son las 07:44 en Costa Rica.
        val hora = horaIngreso("2026-10-27T13:44:00Z", costaRica, hoy = LocalDate.of(2026, 10, 27))
        assertEquals(HoraIngreso("07:44", deOtroDia = false), hora)
    }

    @Test
    fun `un ingreso de otro dia lleva el dia delante y se marca`() {
        val hora = horaIngreso("2026-10-26T23:10:00-06:00", costaRica, hoy = LocalDate.of(2026, 10, 27))
        assertEquals(HoraIngreso("26/10 23:10", deOtroDia = true), hora)
    }

    @Test
    fun `una hora malformada no derriba la tarjeta`() {
        assertEquals(HoraIngreso("—", deOtroDia = false), horaIngreso("x", costaRica, hoy = LocalDate.of(2026, 10, 27)))
    }
}
