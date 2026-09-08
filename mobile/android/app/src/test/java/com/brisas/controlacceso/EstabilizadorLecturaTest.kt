package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class EstabilizadorLecturaTest {

    private val licenciaTexto = "Licencia de Conducir\nNº: 112340567\nVencimiento 03-04-2026"

    private val td1Valido = """
        IDCRI9998887774<<<<<<<<<<<<<<<
        9001011F3001019NIC<<<<<<<<<<<8
        PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
    """.trimIndent()

    private val td1Corrupto = """
        IDCRI9998887784<<<<<<<<<<<<<<<
        9001011F3001019NIC<<<<<<<<<<<8
        PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
    """.trimIndent()

    @Test
    fun sinTextoQuedaBuscando() {
        val r = EstabilizadorLectura().procesarFrame("")
        assertEquals(EstadoEscaneo.BUSCANDO, r.estado)
        assertNull(r.documento)
    }

    @Test
    fun textoSinPalabrasClaveEsInvalido() {
        val r = EstabilizadorLectura().procesarFrame("Un papel cualquiera sin ningun documento reconocible")
        assertEquals(EstadoEscaneo.INVALIDO, r.estado)
    }

    @Test
    fun mrzValidoConfirmaEnUnSoloFrame() {
        // Con checksum disponible no hace falta esperar repeticiones -- eso
        // es justo el punto de usar el dígito verificador (plan, sección 5).
        val estabilizador = EstabilizadorLectura(framesRequeridos = 3)
        val r = estabilizador.procesarFrame(td1Valido)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals("999888777", r.documento?.numeroDocumento)
        assertEquals(FuenteDatos.MRZ, r.documento?.fuenteDatos)
    }

    @Test
    fun mrzCorruptoNuncaConfirmaAunqueSeRepita() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 3)
        repeat(5) {
            val r = estabilizador.procesarFrame(td1Corrupto)
            assertEquals(EstadoEscaneo.INVALIDO, r.estado)
            assertNull(r.documento)
        }
    }

    @Test
    fun sinChecksumNoConfirmaEnElPrimerFrame() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 3)
        val r1 = estabilizador.procesarFrame(licenciaTexto)
        assertEquals(EstadoEscaneo.BUSCANDO, r1.estado)
        assertNull(r1.documento)
    }

    @Test
    fun sinChecksumConfirmaTrasFramesConsistentesRequeridos() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 3)
        estabilizador.procesarFrame(licenciaTexto)
        estabilizador.procesarFrame(licenciaTexto)
        val r3 = estabilizador.procesarFrame(licenciaTexto)
        assertEquals(EstadoEscaneo.CONFIRMADO, r3.estado)
        assertEquals("112340567", r3.documento?.numeroDocumento)
    }

    @Test
    fun cambiarDeCandidatoReiniciaElConteo() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 3)
        estabilizador.procesarFrame(licenciaTexto)
        estabilizador.procesarFrame(licenciaTexto)
        // Un frame con otro número (ej. reflejo cambió un dígito leído) --
        // no debe heredar las repeticiones acumuladas del candidato previo.
        val otraLicencia = "Licencia de Conducir\nNº: 999888777\nVencimiento 03-04-2026"
        val r3 = estabilizador.procesarFrame(otraLicencia)
        assertEquals(EstadoEscaneo.BUSCANDO, r3.estado)
    }

    @Test
    fun tipoConocidoSinNumeroExtraibleQuedaBuscandoNoInvalido() {
        // Palabras clave de licencia presentes, pero sin número legible
        // todavía (glare/ángulo) -- es lectura parcial, no un documento
        // desconocido.
        val texto = "Licencia de Conducir\nVencimiento 03-04-2026"
        val r = EstabilizadorLectura().procesarFrame(texto)
        assertEquals(EstadoEscaneo.BUSCANDO, r.estado)
    }

    // --- Feedback del tipo detectado (nunca se elige a mano) ---

    @Test
    fun mensajeNombraElTipoDetectadoAunqueTodavíaNoConfirme() {
        val texto = "Licencia de Conducir\nVencimiento 03-04-2026"
        val r = EstabilizadorLectura().procesarFrame(texto)
        assertEquals("Licencia de conducir detectado — mantenga firme", r.mensaje)
    }

    @Test
    fun mensajeDeConfirmacionNombraElTipoDetectadoPorFrente() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1)
        val r = estabilizador.procesarFrame(licenciaTexto)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals("Licencia de conducir confirmado", r.mensaje)
    }

    @Test
    fun mensajeDeConfirmacionNombraElTipoDetectadoPorMrz() {
        val r = EstabilizadorLectura().procesarFrame(td1Valido)
        assertEquals("Cédula de residencia (DIMEX) confirmado", r.mensaje)
    }
}
