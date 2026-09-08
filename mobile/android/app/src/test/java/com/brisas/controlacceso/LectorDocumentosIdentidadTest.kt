package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LectorDocumentosIdentidadTest {

    @Test
    fun clasificaCedulaNacional() {
        val texto = "TRIBUNAL SUPREMO DE ELECCIONES\n1-1234-0567\nCOSTA RICA"
        assertEquals(TipoDocumento.CEDULA_NACIONAL, clasificarTipoDocumento(texto))
    }

    @Test
    fun clasificaCedulaResidencia() {
        val texto = "DIRECCIÓN GENERAL DE MIGRACIÓN Y EXTRANJERÍA\nRESIDENTE PERMANENTE"
        assertEquals(TipoDocumento.CEDULA_RESIDENCIA, clasificarTipoDocumento(texto))
    }

    @Test
    fun clasificaLicenciaNacional() {
        val texto = "Licencia de Conducir\nNº: 112340567"
        assertEquals(TipoDocumento.LICENCIA_NACIONAL, clasificarTipoDocumento(texto))
    }

    @Test
    fun clasificaLicenciaExtranjero() {
        val texto = "Licencia de Conducir\nNº: DM-999888777"
        assertEquals(TipoDocumento.LICENCIA_EXTRANJERO, clasificarTipoDocumento(texto))
    }

    @Test
    fun clasificaDesconocidoSinSenales() {
        val texto = "Documento sin ninguna palabra clave reconocible"
        assertEquals(TipoDocumento.DESCONOCIDO, clasificarTipoDocumento(texto))
    }

    // --- Caso central del plan: Documento No. vs Expediente No. ---

    @Test
    fun dimexExtraeDocumentoNoExpediente() {
        val texto = """
            DIRECCIÓN GENERAL DE MIGRACIÓN Y EXTRANJERÍA
            REPÚBLICA DE COSTA RICA
            RESIDENTE PERMANENTE
            LIBRE CONDICIÓN
            Apellidos:
            PEREZ RAMIREZ
            Nombre:
            MARIA JOSE
            Nacionalidad:
            NICARAGUA
            Documento No.: 999888777
            Expediente No.: 135-453544
            Vence: 28 07 2026
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CEDULA_RESIDENCIA, doc?.tipo)
        assertEquals("999888777", doc?.numeroDocumento)
        assertEquals("MARIA JOSE", doc?.nombre)
        assertEquals("PEREZ RAMIREZ", doc?.apellidos)
        assertEquals("NICARAGUA", doc?.nacionalidad)
        assertEquals(FechaDocumento(28, 7, 2026), doc?.vencimiento)
    }

    @Test
    fun dimexNoConfundeConNumeroDeExpedienteSiApareceAntes() {
        val texto = "Expediente No.: 135-453544\nDocumento No.: 999888777"
        val doc = leerDocumentoDeTexto(texto)
        assertEquals("999888777", doc?.numeroDocumento)
    }

    // --- Licencias ---

    @Test
    fun licenciaNacionalNoMarcaExtranjero() {
        val texto = "Licencia de Conducir\nNº: 112340567\nVencimiento 03-04-2026"
        val doc = leerDocumentoDeTexto(texto)
        assertEquals(TipoDocumento.LICENCIA_NACIONAL, doc?.tipo)
        assertEquals("112340567", doc?.numeroDocumento)
        assertEquals(false, doc?.esExtranjero)
    }

    @Test
    fun licenciaExtranjeroRemuevePrefijoDM() {
        val texto = "Licencia de Conducir\nNº: DM-999888777\nVencimiento 03-04-2026"
        val doc = leerDocumentoDeTexto(texto)
        assertEquals(TipoDocumento.LICENCIA_EXTRANJERO, doc?.tipo)
        assertEquals("999888777", doc?.numeroDocumento)
        assertTrue(doc?.esExtranjero == true)
        assertEquals(FechaDocumento(3, 4, 2026), doc?.vencimiento)
    }

    // --- Vigencia ---

    @Test
    fun documentoVigenteSiVencimientoEsFuturo() {
        val vencimiento = FechaDocumento(28, 7, 2026)
        val hoy = FechaDocumento(8, 9, 2025)
        assertEquals(false, vencimiento.estaVencida(hoy))
    }

    @Test
    fun documentoVencidoSiVencimientoEsPasado() {
        val vencimiento = FechaDocumento(28, 7, 2024)
        val hoy = FechaDocumento(8, 9, 2026)
        assertEquals(true, vencimiento.estaVencida(hoy))
    }

    // --- Casos sin match ---

    @Test
    fun devuelveNullSiNoHaySuficienteInformacion() {
        assertNull(leerDocumentoDeTexto("Documento sin numeros completos"))
    }
}
