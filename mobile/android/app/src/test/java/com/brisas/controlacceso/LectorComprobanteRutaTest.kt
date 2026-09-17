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
        Material Descripción Cant.Tot Botelleo Descarga BOT / PZA
        164145 4 Pack Powerade Zero mixto 591 8
        167251 POWERADE ZERO ION 4 591ML MIXE 3
        167264 POWERADE ZERO ION 4 591ML UVA 3
        163966 Powerade ION4 Mountain Blast S 1
        164079 Powerade Zero Frutas 591ml 12U 2
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
    fun extraeTransporteAunqueMlKitLoPartaEnDosLineas() {
        // Bug real reportado en el Samsung A25 (2026-09-15, segunda
        // ronda): ML Kit no siempre entrega "Transporte:" y su valor en la
        // misma línea reconocida -- exigir cero saltos de línea (fix de la
        // primera ronda) dejó de reconocer el comprobante por completo.
        val texto = comprobantePrincipal.replace("Transporte: 700101452", "Transporte:\n700101452")
        val resultado = extraerComprobanteRuta(texto)
        assertEquals("700101452", resultado?.numeroDocumento)
    }

    @Test
    fun noConfundeElTransporteConUnSkuDeLaTablaDeMateriales() {
        // Bug real reportado en el Samsung A25 (2026-09-15): con `\s*` el
        // regex podía cruzar la línea de "Transporte:" y agarrar el primer
        // código de material de la tabla de abajo (`164145`, mismo largo
        // que un número de transporte real) en vez de `700101452`.
        val resultado = extraerComprobanteRuta(comprobantePrincipal)
        assertEquals("700101452", resultado?.numeroDocumento)
        assertTrue("164145" != resultado?.numeroDocumento)
    }

    @Test
    fun noConfirmaUnSkuDeSeisDigitosPegadoDirectoATransporte() {
        // Tercera ronda (2026-09-15): en foto real, ML Kit no siempre
        // ordena las líneas como en el papel -- a veces el salto de línea
        // que el regex tolera cae justo antes de la tabla de materiales
        // (saltándose "Fecha de Entrega:"/"Camión:") en vez de antes de
        // esas etiquetas. Con mínimo 6 dígitos eso confirmaba el primer
        // SKU (`164145`) como si fuera el transporte real. Con mínimo 7,
        // ese frame roto debe rechazarse (null) -- el escáner sigue
        // esperando un frame limpio en vez de confirmar un dato
        // incorrecto.
        val texto = """
            Ruta / No.de Carga: CRR079/ 00001
            Transporte:
            164145 4 Pack Powerade Zero mixto 591 8
        """.trimIndent()
        assertNull(extraerComprobanteRuta(texto))
    }

    @Test
    fun siExtraeElTransporteRealAunqueElSkuApareceAlLado() {
        // Contraparte del test anterior: cuando SÍ está el transporte real
        // de 9 dígitos (aunque sea en la misma línea/bloque que un SKU de
        // 6), se extrae el correcto -- el fix es por longitud, no por
        // prohibir dígitos cercanos a la tabla.
        val texto = """
            Ruta / No.de Carga: CRR079/ 00001
            Transporte: 700101452
            164145 4 Pack Powerade Zero mixto 591 8
        """.trimIndent()
        assertEquals("700101452", extraerComprobanteRuta(texto)?.numeroDocumento)
    }

    @Test
    fun extraeRutaPrincipal() {
        val resultado = extraerComprobanteRuta(comprobantePrincipal)
        assertEquals("CRR079", resultado?.numeroRuta)
        assertEquals(1, resultado?.subNumero)
        assertEquals("700101452", resultado?.numeroDocumento)
        assertEquals(FechaDocumento(15, 9, 2026), resultado?.fecha)
    }

    // Las 3 fixtures de abajo (H2/H3/H4) usan la tabla de materiales REAL
    // de cada una de las otras 3 fotos compartidas (2026-09-15), no un
    // simple reemplazo de ruta/transporte sobre la tabla de la foto
    // principal -- cada comprobante trae su propia carga (productos y
    // cantidad de filas distintos), y conviene probar el parser contra esa
    // variación real en vez de asumir que todas las tablas se ven igual.

    @Test
    fun extraeH2DeSegundaPagina() {
        // Foto con Ruta/No.de Carga CRR079/ 00002, Transporte 700101453 --
        // tabla de cervezas (MODELO/CORONA/BUDWEISER/HOEGAARDEN/CORONITA).
        val texto = """
            Coca Cola FEMSA
            COMPROBANTE DE CARGA DE RUTA
            CENTRO: RFAA Distribuidora Central
            Ruta / No.de Carga: CRR079/ 00002
            Repartidor: 961190782 CARLOS BALMACEDA ORDOÑEZ
            Usuario: CR05506156 Esteban Charpentier Cespedes
            Estatus: Fin carga
            Transporte: 700101453
            Fecha de Entrega: 15.09.2026
            Material Descripción Cant.Tot Botelleo Descarga BOT / PZA
            165531 MODELO ESP VD 355ML 24 PK 2
            165532 CORONA VD 330ML 24 PK 2
            165534 BUDWEISER VD 355ML 24 PK 1
            165547 HOEGAARDEN VD 330ML 24 PK 1
            165552 CORONA CERO VD 355ML 24PK 1
            165656 CORONITA BOT 207ML 6P 5
        """.trimIndent()
        val resultado = extraerComprobanteRuta(texto)
        assertEquals("CRR079", resultado?.numeroRuta)
        assertEquals(2, resultado?.subNumero)
        assertEquals("700101453", resultado?.numeroDocumento)
    }

    @Test
    fun extraeH3DeTerceraPagina() {
        // Foto con Ruta/No.de Carga CRR079/ 00003, Transporte 700101454.
        val texto = """
            Coca Cola FEMSA
            COMPROBANTE DE CARGA DE RUTA
            CENTRO: RFAA Distribuidora Central
            Ruta / No.de Carga: CRR079/ 00003
            Repartidor: 961190782 CARLOS BALMACEDA ORDOÑEZ
            Usuario: CR05506156 Esteban Charpentier Cespedes
            Estatus: Fin carga
            Transporte: 700101454
            Fecha de Entrega: 15.09.2026
            Material Descripción Cant.Tot Botelleo Descarga BOT / PZA
            164145 4 Pack Powerade Zero mixto 591 1
            164580 4 PACK POWERADE SURTIDO 600ML 2
            167264 POWERADE ZERO ION 4 591ML UVA 1
            163966 Powerade ION4 Mountain Blast S 1
        """.trimIndent()
        val resultado = extraerComprobanteRuta(texto)
        assertEquals(3, resultado?.subNumero)
        assertEquals("700101454", resultado?.numeroDocumento)
    }

    @Test
    fun extraeH4DeCuartaPaginaConFilaDeSubTotal() {
        // Foto con Ruta/No.de Carga CRR079/ 00004, Transporte 700101455 --
        // esta trae una fila "Sub-Total Familia ... 1/0" en medio de la
        // tabla, con su propia barra "/" -- confirma que no se confunde
        // con el patrón de "Ruta / No.de Carga" (que exige esa etiqueta
        // literal antes de la barra, ver REGEX_RUTA_NUMERO_CARGA).
        val texto = """
            Coca Cola FEMSA
            COMPROBANTE DE CARGA DE RUTA
            CENTRO: RFAA Distribuidora Central
            Ruta / No.de Carga: CRR079/ 00004
            Repartidor: 961190782 CARLOS BALMACEDA ORDOÑEZ
            Usuario: CR05506156 Esteban Charpentier Cespedes
            Estatus: Fin carga
            Transporte: 700101455
            Fecha de Entrega: 15.09.2026
            Material Descripción Cant.Tot Botelleo Descarga BOT / PZA
            163966 Powerade ION4 Mountain Blast S 1
            Sub-Total Familia AGUAS/PW/BIB 1/0
            163519 Coca-Cola Sin Azúcar 1.5L PET 11
            168279 FRESCA 1.5L PET 8 UDS TERMO
        """.trimIndent()
        val resultado = extraerComprobanteRuta(texto)
        assertEquals("CRR079", resultado?.numeroRuta)
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
