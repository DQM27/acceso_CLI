package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.LocalDate

class MrzParserTest {

    private fun td1ConNacimiento(fechaNacimiento: String): String {
        val l1 = "IDCRI1011101119<<<<<<<<<<<<<<<"
        val checkNacimiento = digitoVerificadorMrz(fechaNacimiento)
        val vencimiento = "300101"
        val checkVencimiento = digitoVerificadorMrz(vencimiento)
        val opcional2 = "<<<<<<<<<<<"
        val compuesto = l1.substring(5) + fechaNacimiento + checkNacimiento +
            vencimiento + checkVencimiento + opcional2
        val checkCompuesto = digitoVerificadorMrz(compuesto)
        val l2 = "$fechaNacimiento${checkNacimiento}F$vencimiento$checkVencimiento" +
            "CRI$opcional2$checkCompuesto"
        return listOf(l1, l2, "PEREZ<<MARIA<JOSE<<<<<<<<<<<<<").joinToString("\n")
    }

    // Checksum verificado con script Python (algoritmo ICAO 9303 real) --
    // ver docs/arquitectura/fixtures-ocr-sinteticos.md sección 4. Datos inventados.
    private val td1Valido = """
        C<CRI9998887774<<<<<<<<<<<<<<<
        9001011F3001019NIC<<<<<<<<<<<8
        PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
    """.trimIndent()

    // Mismo caso, con un dígito alterado en el número de documento (línea 1).
    private val td1Corrupto = """
        C<CRI9998887784<<<<<<<<<<<<<<<
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
    fun noCombinaLineasMrzQueNoSonConsecutivas() {
        val separado = td1Valido.lines().let { lineas ->
            listOf(lineas[0], "TEXTO INTERMEDIO", lineas[1], lineas[2]).joinToString("\n")
        }
        assertNull(parsearMrzTd1(separado))
    }

    @Test
    fun sigloDeNacimientoDependeDeLaFechaRealNoDeUnaConstante() {
        val mrz = td1ConNacimiento("270101")

        assertEquals(
            FechaDocumento(1, 1, 1927),
            parsearMrzTd1(mrz, LocalDate.of(2026, 12, 31))?.fechaNacimiento,
        )
        assertEquals(
            FechaDocumento(1, 1, 2027),
            parsearMrzTd1(mrz, LocalDate.of(2027, 1, 1))?.fechaNacimiento,
        )
    }

    @Test
    fun fechaMrzInexistenteNoSeExponeComoFechaValida() {
        val resultado = parsearMrzTd1(td1ConNacimiento("900231"))
        assertTrue(resultado?.checksumsValidos == true)
        assertNull(resultado?.fechaNacimiento)
    }

    @Test
    fun ignoraEspaciosQueElOcrPuedeInsertarEnLaMrz() {
        // Mismo contenido que td1Valido, con espacios de más como los que
        // ML Kit a veces mete al leer la banda MRZ.
        val conRuido = """
            C<CRI 9998887774<<<<<<<<<<<<<<<
            9001011F3001019NIC<<<<<<<<<<<8
            PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
        """.trimIndent()
        val resultado = parsearMrzTd1(conRuido)
        assertEquals("999888777", resultado?.numeroDocumento)
        assertTrue(resultado?.checksumsValidos == true)
    }

    @Test
    fun resuelveNumeroExtendidoDeDimexCostarricense() {
        // Convención costarricense (perfil CR_DIMEX_TD1_2023): la posición
        // 15 SÍ es el check digit normal del bloque de 9 dígitos, y el resto
        // del DIMEX (11-12 dígitos en total) sigue como dígitos nacionales
        // sin check digit propio en el campo opcional -- distinto del
        // mecanismo "long document number" de ICAO (ese exige '<' en la
        // posición 15). Datos inventados, pero estructura y checksums
        // (incluido el compuesto) verificados con el mismo algoritmo contra
        // un DIMEX real que sí trae este patrón.
        val td1Dimex = """
            C<CRI1999888772701<<<<<<<<<<<<
            9001011M3001019NIC<<<<<<<<<<<0
            PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
        """.trimIndent()

        val resultado = parsearMrzTd1(td1Dimex)

        assertEquals(false, resultado?.numeroDocumentoExtendidoSinSoporte)
        assertEquals("199988877701", resultado?.numeroDocumento)
        assertTrue(resultado?.checksumsValidos == true)
    }

    @Test
    fun dimexConCheckDigitDelBloqueBaseAlteradoNoConfirma() {
        val td1DimexCorrupto = """
            C<CRI1999888773701<<<<<<<<<<<<
            9001011M3001019NIC<<<<<<<<<<<0
            PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
        """.trimIndent()
        val resultado = parsearMrzTd1(td1DimexCorrupto)
        assertEquals(false, resultado?.checksumsValidos)
    }

    @Test
    fun digitoEnPosicion15SinChecksumValidoYSinSerCriQuedaSinSoporte() {
        // Mismo patrón visual (dígito + más dígitos en el opcional) pero de
        // otro país -- no se asume que siga la convención de Costa Rica sin
        // verificarlo, se marca explícitamente en vez de arriesgar un
        // número mal reconstruido.
        val td1OtroPais = """
            C<ARG1999888772701<<<<<<<<<<<<
            9001011M3001019ARG<<<<<<<<<<<0
            PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
        """.trimIndent()
        val resultado = parsearMrzTd1(td1OtroPais)
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
