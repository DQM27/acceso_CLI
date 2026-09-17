package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/// Fixtures reconstruidos a partir de las notas guardadas en
/// `docs/arquitectura/muestras-ocr-aisladas.md` (fotos del 2026-09-08,
/// aisladas en su momento, activadas ahora para el paso "Gafete KOF") --
/// a diferencia de `LectorComprobanteRutaTest.kt`, estas NO son
/// transcripciones línea por línea de una foto fresca, son una
/// reconstrucción razonable de esas notas. Falta la validación real
/// contra el carnet físico (overlay debug, mismo camino que ya se usó
/// para el comprobante).
class LectorCarnetKofTest {

    private val textoFrente = """
        COCA-COLA FEMSA
        BRASLY DANIEL
        CHAVES BONILLA
        5 AÑOS
        COCA-COLA FEMSA
    """.trimIndent()

    private val textoReverso = """
        CENTRAL COSTA RICA DE ALERTA Y RESPUESTA
        800-2256327
        5040017
    """.trimIndent()

    @Test
    fun clasificaElFrenteComoCarnetKof() {
        assertTrue(esCarnetKof(textoFrente))
    }

    @Test
    fun clasificaElReversoComoCarnetKof() {
        assertTrue(esCarnetKof(textoReverso))
    }

    @Test
    fun noClasificaElComprobanteDeCargaAunqueComparteLaMarcaFemsa() {
        val textoComprobante = """
            Coca Cola FEMSA
            COMPROBANTE DE CARGA DE RUTA
            Ruta / No.de Carga: CRR079/ 00001
            Transporte: 700101452
        """.trimIndent()
        assertFalse(esCarnetKof(textoComprobante))
    }

    @Test
    fun noClasificaTextoAjeno() {
        assertFalse(esCarnetKof("Licencia de Conducir\nNº: 112340567"))
    }

    @Test
    fun extraeElNombreDelFrente() {
        val resultado = extraerCarnetKof(textoFrente)
        assertEquals("BRASLY DANIEL CHAVES BONILLA", resultado?.nombre)
        assertNull(resultado?.codigoEmpleado)
    }

    @Test
    fun extraeElCodigoDeEmpleadoDelReverso() {
        val resultado = extraerCarnetKof(textoReverso)
        assertEquals("5040017", resultado?.codigoEmpleado)
        assertNull(resultado?.nombre)
    }

    @Test
    fun noConfundeElCodigoDeEmpleadoConElTelefonoDeEmergencia() {
        // Bug real atrapado al escribir este test: "800-2256327" esconde
        // una corrida de 7 dígitos ("2256327") que un `\b\d{7}\b` suelto
        // confirmaba como si fuera el código de empleado -- el teléfono
        // aparece antes que el código real en el texto del reverso.
        val resultado = extraerCarnetKof(textoReverso)
        assertEquals("5040017", resultado?.codigoEmpleado)
        assertTrue("2256327" != resultado?.codigoEmpleado)
    }

    @Test
    fun aceptaCodigosDeEmpleadoDeCincoYSeisDigitos() {
        // Segunda corrección (2026-09-15), contra `empleados_costa_rica.sql`
        // (1438 filas reales): el largo del código NO es fijo en 7 como se
        // asumió con la única muestra vista -- hay 23 códigos de 5 dígitos
        // (ej. `77851`) y 61 de 6 (ej. `330002`), ambos reales en esa base.
        val textoReversoCincoDigitos = textoReverso.replace("5040017", "77851")
        assertEquals("77851", extraerCarnetKof(textoReversoCincoDigitos)?.codigoEmpleado)

        val textoReversoSeisDigitos = textoReverso.replace("5040017", "330002")
        assertEquals("330002", extraerCarnetKof(textoReversoSeisDigitos)?.codigoEmpleado)
    }

    @Test
    fun sinSenalesDeCarnetNoHayResultado() {
        assertNull(extraerCarnetKof("texto cualquiera sin relación"))
    }
}
