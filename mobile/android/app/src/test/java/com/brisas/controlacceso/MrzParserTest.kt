package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MrzParserTest {

    // Checksum verificado con script Python (algoritmo ICAO 9303 real) --
    // ver fixtures-ocr-sinteticos.md sección 4. Datos inventados.
    private val td1Valido = """
        IDCRI9998887774<<<<<<<<<<<<<<<
        9001011F3001019NIC<<<<<<<<<<<8
        PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
    """.trimIndent()

    // Mismo caso, con un dígito alterado en el número de documento (línea 1).
    private val td1Corrupto = """
        IDCRI9998887784<<<<<<<<<<<<<<<
        9001011F3001019NIC<<<<<<<<<<<8
        PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
    """.trimIndent()

    @Test
    fun digitoVerificadorCoincideConAlgoritmoIcao() {
        // "999888777" -> dígito verificador 4, calculado independientemente
        // con Python (pesos 7,3,1, A-Z=10-35, '<'=0).
        assertEquals(4, digitoVerificadorMrz("999888777"))
    }

    @Test
    fun parseaTd1ValidoCompleto() {
        val resultado = parsearMrzTd1(td1Valido)

        assertEquals("TD1", resultado?.formato)
        assertEquals("CRI", resultado?.paisEmisor)
        assertEquals("999888777", resultado?.numeroDocumento)
        assertEquals("PEREZ", resultado?.apellidos)
        assertEquals("MARIA JOSE", resultado?.nombres)
        assertEquals("NIC", resultado?.nacionalidad)
        assertEquals('F', resultado?.sexo)
        assertEquals(FechaDocumento(1, 1, 1990), resultado?.fechaNacimiento)
        assertEquals(FechaDocumento(1, 1, 2030), resultado?.fechaVencimiento)
        assertTrue(resultado?.checksumsValidos == true)
    }

    @Test
    fun rechazaTd1ConDigitoAlterado() {
        val resultado = parsearMrzTd1(td1Corrupto)

        // Se parsea (las líneas calzan en formato) pero el checksum debe
        // fallar -- este es el caso central de la sección 5 del plan: nunca
        // aceptar en silencio un MRZ cuyo dígito verificador no calza.
        assertEquals(false, resultado?.checksumsValidos)
    }

    @Test
    fun devuelveNullSiNoHayTresLineasMrz() {
        assertNull(parsearMrzTd1("esto no es un MRZ\nsolo texto normal del frente"))
    }

    @Test
    fun ignoraEspaciosQueElOcrPuedeInsertarEnLaMrz() {
        // Mismo contenido que td1Valido, con espacios de más como los que
        // ML Kit a veces mete al leer la banda MRZ.
        val conRuido = """
            IDCRI 9998887774<<<<<<<<<<<<<<<
            9001011F3001019NIC<<<<<<<<<<<8
            PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
        """.trimIndent()
        val resultado = parsearMrzTd1(conRuido)
        assertEquals("999888777", resultado?.numeroDocumento)
        assertTrue(resultado?.checksumsValidos == true)
    }

    @Test
    fun detectaNumeroExtendidoNoEstandarSinCalcularChecksumIncorrecto() {
        // Caso real de DIMEX: dígito (no relleno '<') en la posición 15 y
        // más dígitos del número siguen en el área opcional -- no sigue el
        // mecanismo estándar de ICAO y no está verificado contra una
        // referencia confiable. Debe marcarse explícitamente en vez de
        // fingir un checksum válido con un algoritmo sin confirmar.
        val td1NumeroLargo = """
            IDCRI1558243956105<<<<<<<<<<<<
            9001011F3001019NIC<<<<<<<<<<<8
            PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
        """.trimIndent()
        val resultado = parsearMrzTd1(td1NumeroLargo)
        assertTrue(resultado?.numeroDocumentoExtendidoSinSoporte == true)
        assertEquals(false, resultado?.checksumsValidos)
    }

    @Test
    fun parseaNumeroExtendidoEstandarIcaoConChecksumValido() {
        // Mecanismo estándar ICAO 9303: posición 15 = '<', continuación +
        // check digit en el campo opcional. Fixture verificado contra un
        // caso real documentado (cédula belga, issue #4 de Arg0s1080/mrz) y
        // recalculado con el mismo algoritmo de checksum vía Python.
        val td1ExtendidoEstandar = """
            IDBEL123456789<1233<<<<<<<<<<<
            9001011F3001019BEL<<<<<<<<<<<8
            PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
        """.trimIndent()

        val resultado = parsearMrzTd1(td1ExtendidoEstandar)

        assertEquals(false, resultado?.numeroDocumentoExtendidoSinSoporte)
        assertEquals("123456789123", resultado?.numeroDocumento)
        assertTrue(resultado?.checksumsValidos == true)
    }

    @Test
    fun rechazaNumeroExtendidoEstandarConCheckDigitAlterado() {
        val td1Corrupto = """
            IDBEL123456789<1234<<<<<<<<<<<
            9001011F3001019BEL<<<<<<<<<<<8
            PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
        """.trimIndent()
        val resultado = parsearMrzTd1(td1Corrupto)
        assertEquals(false, resultado?.numeroDocumentoExtendidoSinSoporte)
        assertEquals(false, resultado?.checksumsValidos)
    }

    // --- TD3 (pasaporte) ---

    @Test
    fun parseaTd3ValidoCompleto() {
        val td3Valido = """
            P<CRIPEREZ<<MARIA<JOSE<<<<<<<<<<<<<<<<<<<<<<
            A1234567<6CRI9001011F3001019<<<<<<<<<<<<<<04
        """.trimIndent()

        val resultado = parsearMrzTd3(td3Valido)

        assertEquals("TD3", resultado?.formato)
        assertEquals("CRI", resultado?.paisEmisor)
        assertEquals("A1234567", resultado?.numeroDocumento)
        assertEquals("PEREZ", resultado?.apellidos)
        assertEquals("MARIA JOSE", resultado?.nombres)
        assertTrue(resultado?.checksumsValidos == true)
    }

    @Test
    fun leerMrzDeTextoPruebaTd1AntesQueTd3() {
        val resultado = leerMrzDeTexto(td1Valido)
        assertEquals("TD1", resultado?.formato)
    }
}
