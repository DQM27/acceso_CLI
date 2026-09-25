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
/// confusables. Lo único que sigue del lado Kotlin es `buscarLineasMrz`
/// (extracción mecánica de líneas, privada a este archivo) -- se ejercita
/// acá sólo indirectamente, a través del punto de entrada público
/// `leerMrzDeTexto`, que es lo único invocable desde otro archivo.
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
}
