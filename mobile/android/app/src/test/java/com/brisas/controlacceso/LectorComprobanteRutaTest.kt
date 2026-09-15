package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/// Fixtures basados en las 4 fotos reales de "Comprobante de Carga de
/// Ruta" (Coca-Cola FEMSA) compartidas por el usuario el 2026-09-15 --
/// misma ruta (CRR079), sub-número y "Transporte" distintos por página
/// (ver docs/planes-implementados/plan-control-rutas.md).
class LectorComprobanteRutaTest {

    private val comprobantePrincipal = """
        Coca Cola FEMSA
        COMPROBANTE DE CARGA DE RUTA
        CENTRO: RFAA Distribuidora Central
        Ruta / No.de Carga: CRR079/ 00001
        Repartidor: 961190782 CARLOS BALMACEDA ORDOÑEZ
        Usuario: CR05506156 Esteban Charpentier Cespedes
        Estatus: Fin carga
        Transporte: 700101452
        Fecha de Entrega: 15.09.2026
        Camión: CRDUMMY
    """.trimIndent()

    @Test
    fun clasificaComprobanteDeCargaDeRuta() {
        assertTrue(esComprobanteCargaRuta(comprobantePrincipal))
    }

    @Test
    fun noClasificaTextoAjeno() {
        assertFalse(esComprobanteCargaRuta("Licencia de Conducir\nNº: 112340567"))
    }

    @Test
    fun extraeRutaPrincipal() {
        val resultado = extraerComprobanteRuta(comprobantePrincipal)
        assertEquals("CRR079", resultado?.numeroRuta)
        assertEquals(1, resultado?.subNumero)
        assertEquals("700101452", resultado?.numeroDocumento)
        assertEquals(FechaDocumento(15, 9, 2026), resultado?.fecha)
    }

    @Test
    fun extraeH2DeSegundaPagina() {
        val texto = comprobantePrincipal
            .replace("CRR079/ 00001", "CRR079/ 00002")
            .replace("700101452", "700101453")
        val resultado = extraerComprobanteRuta(texto)
        assertEquals("CRR079", resultado?.numeroRuta)
        assertEquals(2, resultado?.subNumero)
        assertEquals("700101453", resultado?.numeroDocumento)
    }

    @Test
    fun extraeH3DeTerceraPagina() {
        val texto = comprobantePrincipal
            .replace("CRR079/ 00001", "CRR079/ 00003")
            .replace("700101452", "700101454")
        val resultado = extraerComprobanteRuta(texto)
        assertEquals(3, resultado?.subNumero)
        assertEquals("700101454", resultado?.numeroDocumento)
    }

    @Test
    fun extraeH4DeCuartaPagina() {
        val texto = comprobantePrincipal
            .replace("CRR079/ 00001", "CRR079/ 00004")
            .replace("700101452", "700101455")
        val resultado = extraerComprobanteRuta(texto)
        assertEquals(4, resultado?.subNumero)
        assertEquals("700101455", resultado?.numeroDocumento)
    }

    @Test
    fun sinTransporteNoHayResultado() {
        val texto = comprobantePrincipal.lineToRemoveContaining("Transporte:")
        assertNull(extraerComprobanteRuta(texto))
    }

    @Test
    fun sinRutaNoHayResultado() {
        val texto = comprobantePrincipal.lineToRemoveContaining("Ruta / No.de Carga:")
        assertNull(extraerComprobanteRuta(texto))
    }

    @Test
    fun sinFechaSigueExtrayendoElRestoConFechaNula() {
        val texto = comprobantePrincipal.lineToRemoveContaining("Fecha de Entrega:")
        val resultado = extraerComprobanteRuta(texto)
        assertEquals("CRR079", resultado?.numeroRuta)
        assertEquals("700101452", resultado?.numeroDocumento)
        assertNull(resultado?.fecha)
    }

    @Test
    fun toleraFormatoNoDeCargaConEspacio() {
        // Variante con "No. de Carga" (con espacio) en vez de "No.de Carga"
        // -- ambas grafías son plausibles al imprimir/leer el mismo campo.
        val texto = comprobantePrincipal.replace("No.de Carga", "No. de Carga")
        val resultado = extraerComprobanteRuta(texto)
        assertEquals("CRR079", resultado?.numeroRuta)
        assertEquals(1, resultado?.subNumero)
    }

    private fun String.lineToRemoveContaining(fragmento: String): String =
        lines().filterNot { fragmento in it }.joinToString("\n")
}
