package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CedulaOcrTest {
    @Test
    fun extraeCedulaConGuiones() {
        assertEquals("112340567", extraerCedulaDeTexto("CEDULA IDENTIDAD\n1-1234-0567\nCOSTA RICA"))
    }

    @Test
    fun extraeCedulaConEspacios() {
        assertEquals("112340567", extraerCedulaDeTexto("Identificacion 1 1234 0567"))
    }

    @Test
    fun extraeCedulaPegada() {
        assertEquals("112340567", extraerCedulaDeTexto("CR 112340567"))
    }

    @Test
    fun ignoraTextoSinCedula() {
        assertNull(extraerCedulaDeTexto("Documento sin numeros completos"))
    }

    @Test
    fun rechazaNumeroQueEmpiezaEnCeroPorqueNingunaProvinciaEsCero() {
        assertNull(extraerCedulaDeTexto("001234567"))
        assertNull(extraerCedulaDeTexto("0 1234 5678"))
    }

    @Test
    fun saltaElNumeroDeControlYSigueHastaLaCedulaReal() {
        // Antes se devolvía el primero ("001234567") y quien llamaba lo
        // descartaba, perdiendo la cédula que venía después.
        assertEquals("123456789", extraerCedulaDeTexto("001234567\n123456789"))
    }

    @Test
    fun ignoraNumeroDeExpedienteConFormatoDeCedula() {
        assertNull(extraerCedulaDeTexto("Expediente No.: 1-2345-6789"))
    }
}
