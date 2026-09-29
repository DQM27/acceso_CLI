package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import uniffi.control_acceso_mobile.FormatoMrz

/// Los 15 casos de parseo/checksum/número extendido que antes vivían acá
/// migraron 1:1 a `mobile/rust-core/src/mrz.rs` (`#[cfg(test)] mod tests`)
/// junto con la migración a Rust del 2026-09-25 (ver
/// `docs/auditorias/auditoria-separacion-kotlin-rust-2026-09-25.md`) --
/// incluidos los dos casos reales documentados (DIMEX costarricense, cédula
/// belga issue Arg0s1080/mrz#4) y los nuevos de corrección acotada de
/// confusables. Desde la auditoría OCR 2026-09-28 (A-1) también la
/// búsqueda de las líneas dentro del texto vive en Rust
/// (`lectura_documentos/mrz_texto.rs`): estos casos la ejercitan a través
/// del punto de entrada de siempre, `leerMrzDeTexto`.
class MrzParserTest {

    // Checksum verificado con script Python (algoritmo ICAO 9303 real) --
    // ver docs/arquitectura/fixtures-ocr-sinteticos.md sección 4. Datos inventados.
    private val td1Valido = """
        C<CRI9998887774<<<<<<<<<<<<<<<
        9001011F3001019NIC<<<<<<<<<<<8
        PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
    """.trimIndent()

    @Test
    fun ubicaYManda3LineasTd1ConsecutivasAlParserDeRust() {
        val texto = "texto de ruido antes\n$td1Valido\nruido después"

        val resultado = leerMrzDeTexto(texto, java.time.LocalDate.of(2026, 1, 1))

        assertEquals(FormatoMrz.TD1, resultado?.formato)
        assertEquals("999888777", resultado?.numeroDocumento)
        assertEquals(true, resultado?.checksumsValidos)
    }

    @Test
    fun noEncuentraLineasMrzEnTextoSinFormaDeMrz() {
        val resultado = leerMrzDeTexto("esto no es un MRZ\nsolo texto normal del frente")
        assertNull(resultado)
    }

    @Test
    fun noCombinaLineasMrzQueNoSonConsecutivas() {
        val separado = td1Valido.lines().let { lineas ->
            listOf(lineas[0], "TEXTO INTERMEDIO", lineas[1], lineas[2]).joinToString("\n")
        }
        assertNull(leerMrzDeTexto(separado))
    }

    @Test
    fun ignoraEspaciosQueElOcrPuedeInsertarEnLaMrz() {
        val conRuido = """
            C<CRI 9998887774<<<<<<<<<<<<<<<
            9001011F3001019NIC<<<<<<<<<<<8
            PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
        """.trimIndent()
        val resultado = leerMrzDeTexto(conRuido, java.time.LocalDate.of(2026, 1, 1))
        assertEquals("999888777", resultado?.numeroDocumento)
        assertEquals(true, resultado?.checksumsValidos)
    }

    private val hoy = java.time.LocalDate.of(2026, 1, 1)

    @Test
    fun toleraUnRellenoPerdidoEnLaLinea1() {
        val lineas = td1Valido.lines()
        // ML Kit se comió un '<' de la corrida de relleno (29 caracteres).
        val texto = listOf(lineas[0].dropLast(1), lineas[1], lineas[2]).joinToString("\n")
        val resultado = leerMrzDeTexto(texto, hoy)
        assertEquals("999888777", resultado?.numeroDocumento)
        assertEquals(true, resultado?.checksumsValidos)
    }

    @Test
    fun toleraUnRellenoDeMasEnLaLinea2SinTocarElDigitoVerificadorFinal() {
        val lineas = td1Valido.lines()
        // 31 caracteres: un '<' duplicado en el relleno opcional, antes del
        // dígito verificador compuesto (el '8' final).
        val linea2 = lineas[1].replace("NIC<<<", "NIC<<<<")
        val texto = listOf(lineas[0], linea2, lineas[2]).joinToString("\n")
        val resultado = leerMrzDeTexto(texto, hoy)
        assertEquals(true, resultado?.checksumsValidos)
    }

    @Test
    fun toleraRellenoLeidoComoKYComoComillasAngulares() {
        val lineas = td1Valido.lines()
        val texto = listOf(
            lineas[0].replace("<<<<<<<<", "KKKKKKKK"),
            lineas[1].replace("<<<<", "««"),
            lineas[2].replace("PEREZ<<", "PEREZKK"),
        ).joinToString("\n")
        val resultado = leerMrzDeTexto(texto, hoy)
        assertEquals(true, resultado?.checksumsValidos)
        assertEquals("PEREZ", resultado?.apellidos)
        assertEquals("MARIA JOSE", resultado?.nombres)
    }

    @Test
    fun unaLineaConDemasiadosCaracteresDeDiferenciaNoSeUsa() {
        val lineas = td1Valido.lines()
        val texto = listOf(lineas[0].dropLast(4), lineas[1], lineas[2]).joinToString("\n")
        assertNull(leerMrzDeTexto(texto, hoy))
    }
}
