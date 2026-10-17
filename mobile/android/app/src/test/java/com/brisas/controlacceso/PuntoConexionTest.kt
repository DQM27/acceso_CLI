package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test

class PuntoConexionTest {
    @Test
    fun `con red y sin fallo esta en linea`() {
        assertEquals(EstadoConexion.EN_LINEA, estadoConexion(hayRed = true, falloSincronizacion = false))
    }

    @Test
    fun `con red y un fallo de sincronizacion avisa el error`() {
        assertEquals(
            EstadoConexion.ERROR_SINCRONIZACION,
            estadoConexion(hayRed = true, falloSincronizacion = true),
        )
    }

    @Test
    fun `sin red manda sobre el error de sincronizacion`() {
        assertEquals(EstadoConexion.SIN_RED, estadoConexion(hayRed = false, falloSincronizacion = true))
        assertEquals(EstadoConexion.SIN_RED, estadoConexion(hayRed = false, falloSincronizacion = false))
    }
}
