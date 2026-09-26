package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class EstabilizadorLecturaTest {

    private val licenciaTexto = "Licencia de Conducir\nNº: 112340567\nVencimiento 03-04-2030"

    private val td1Valido = """
        C<CRI9998887774<<<<<<<<<<<<<<<
        9001011F3001019NIC<<<<<<<<<<<8
        PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
    """.trimIndent()

    private val td1Corrupto = """
        C<CRI9998887784<<<<<<<<<<<<<<<
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
    fun numeroDeNueveDigitosSinSenalesNoSeAceptaComoCedula() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 2)
        repeat(3) {
            val r = estabilizador.procesarFrame("Teléfono de contacto 888888888")
            assertEquals(EstadoEscaneo.INVALIDO, r.estado)
            assertNull(r.documento)
        }
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
    fun mrzValidoDeTipoNoSoportadoNoSeConfirma() {
        val td1Extranjero = """
            C<ARG9998887774<<<<<<<<<<<<<<<
            9001011F3001019NIC<<<<<<<<<<<8
            PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
        """.trimIndent()

        val r = EstabilizadorLectura().procesarFrame(td1Extranjero)

        assertEquals(EstadoEscaneo.INVALIDO, r.estado)
        assertNull(r.documento)
        assertEquals("Documento no soportado", r.mensaje)
    }

    @Test
    fun sinChecksumNoConfirmaEnElPrimerFrame() {
        val estabilizador = EstabilizadorLectura()
        val r1 = estabilizador.procesarFrame(licenciaTexto)
        assertEquals(EstadoEscaneo.BUSCANDO, r1.estado)
        assertNull(r1.documento)
    }

    @Test
    fun sinChecksumConfirmaTrasDosFramesConsistentesPorDefecto() {
        val estabilizador = EstabilizadorLectura()
        estabilizador.procesarFrame(licenciaTexto)
        val r2 = estabilizador.procesarFrame(licenciaTexto)
        assertEquals(EstadoEscaneo.CONFIRMADO, r2.estado)
        assertEquals("112340567", r2.documento?.numeroDocumento)
    }

    @Test
    fun camposOpcionalesIntermitentesNoRompenLaEstabilizacion() {
        val estabilizador = EstabilizadorLectura()
        estabilizador.procesarFrame(licenciaTexto)
        val sinFecha = "Licencia de Conducir\nNº: 112340567"

        val r = estabilizador.procesarFrame(sinFecha)

        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals("112340567", r.documento?.numeroDocumento)
    }

    @Test
    fun cambiarDeCandidatoReiniciaElConteo() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 3)
        estabilizador.procesarFrame(licenciaTexto)
        estabilizador.procesarFrame(licenciaTexto)
        // Un frame con otro número (ej. reflejo cambió un dígito leído) --
        // no debe heredar las repeticiones acumuladas del candidato previo.
        val otraLicencia = "Licencia de Conducir\nNº: 999888777\nVencimiento 03-04-2030"
        val r3 = estabilizador.procesarFrame(otraLicencia)
        assertEquals(EstadoEscaneo.BUSCANDO, r3.estado)
    }

    @Test
    fun tipoConocidoSinNumeroExtraibleQuedaBuscandoNoInvalido() {
        // Palabras clave de licencia presentes, pero sin número legible
        // todavía (glare/ángulo) -- es lectura parcial, no un documento
        // desconocido.
        val texto = "Licencia de Conducir\nVencimiento 03-04-2030"
        val r = EstabilizadorLectura().procesarFrame(texto)
        assertEquals(EstadoEscaneo.BUSCANDO, r.estado)
    }

    // --- Feedback del tipo detectado (nunca se elige a mano) ---

    @Test
    fun mensajeNombraElTipoDetectadoAunqueTodavíaNoConfirme() {
        val texto = "Licencia de Conducir\nVencimiento 03-04-2030"
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

    @Test
    fun modoDocumentoNoAceptaGafeteContratista() {
        val texto = """
            CARNÉ
            PROVISIONAL
            CRC - 16
            CONTRATISTAS
            Costa Rica
        """.trimIndent()

        val r = EstabilizadorLectura(framesRequeridos = 1).procesarFrame(texto)

        assertEquals(EstadoEscaneo.BUSCANDO, r.estado)
        assertNull(r.documento)
        assertEquals("Apunte al documento del contratista", r.mensaje)
    }

    @Test
    fun modoGafeteAceptaSoloGafeteContratista() {
        val texto = """
            CARNÉ
            PROVISIONAL
            CRC - 16
            CONTRATISTAS
            Costa Rica
        """.trimIndent()

        val r = EstabilizadorLectura(
            modo = ModoEscaneoDocumento.GAFETE_CONTRATISTA,
            framesRequeridos = 1,
        ).procesarFrame(texto)

        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(TipoDocumento.GAFETE_CONTRATISTA, r.documento?.tipo)
        assertEquals("16", r.documento?.numeroDocumento)
    }

    @Test
    fun modoGafeteExigeMasRepeticionesQueDocumentoContratistaPorDefecto() {
        // MV-06 (auditoría 2026-09-24): en modo gafete el default sube a 3
        // repeticiones (antes 2, igual que DOCUMENTO_CONTRATISTA) porque acá
        // no hay formulario de revisión -- confirmar dispara una salida real
        // sola. Sin el `framesRequeridos` explícito de
        // `modoGafeteAceptaSoloGafeteContratista` (que fija 1 a propósito
        // para no depender de este default), para probar justo el número
        // que trae la clase por sí sola en este modo.
        val texto = """
            CARNÉ
            PROVISIONAL
            CRC - 16
            CONTRATISTAS
            Costa Rica
        """.trimIndent()
        val estabilizador = EstabilizadorLectura(modo = ModoEscaneoDocumento.GAFETE_CONTRATISTA)

        estabilizador.procesarFrame(texto)
        val segundaRepeticion = estabilizador.procesarFrame(texto)
        assertEquals(EstadoEscaneo.BUSCANDO, segundaRepeticion.estado)

        val terceraRepeticion = estabilizador.procesarFrame(texto)
        assertEquals(EstadoEscaneo.CONFIRMADO, terceraRepeticion.estado)
    }

    @Test
    fun distingueCedulaNacionalDeDimexUsandoSoloElMrz() {
        // Mismo formato TD1 que el DIMEX, distinto código de documento en
        // posiciones 1-2 (ID en vez de C<) -- código real confirmado contra
        // el Decreto TSE n.° 22-2025 (cédula nacional vigente desde
        // oct-2025). Datos numéricos inventados, checksums verificados.
        val td1CedulaNacional = """
            IDCRI1011101119<<<<<<<<<<<<<<<
            9001011F3001019CRI<<<<<<<<<<<8
            PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
        """.trimIndent()
        val r = EstabilizadorLectura().procesarFrame(td1CedulaNacional)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(TipoDocumento.CEDULA_NACIONAL, r.documento?.tipo)
        assertEquals("Cédula de identidad confirmado", r.mensaje)
    }

    // --- Vigencia (fecha inyectada, no depende del reloj real) ---

    @Test
    fun documentoVigenteNoSeMarcaComoVencido() {
        val hoyFijo = FechaDocumento(1, 1, 2026) // antes del 03-04-2030 de licenciaTexto
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1, obtenerFechaHoy = { hoyFijo })
        val r = estabilizador.procesarFrame(licenciaTexto)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(false, r.vencido)
        assertEquals("Licencia de conducir confirmado", r.mensaje)
    }

    @Test
    fun documentoVencidoSeAnunciaEnElMensajeDeConfirmacionPorFrente() {
        val hoyFijo = FechaDocumento(1, 1, 2031) // después del 03-04-2030 de licenciaTexto
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1, obtenerFechaHoy = { hoyFijo })
        val r = estabilizador.procesarFrame(licenciaTexto)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(true, r.vencido)
        assertEquals("Licencia de conducir confirmado — DOCUMENTO VENCIDO", r.mensaje)
    }

    @Test
    fun documentoVencidoSeAnunciaEnElMensajeDeConfirmacionPorMrz() {
        // td1Valido vence 01/01/2030 -- fijamos "hoy" después de esa fecha.
        val hoyFijo = FechaDocumento(1, 1, 2031)
        val estabilizador = EstabilizadorLectura(obtenerFechaHoy = { hoyFijo })
        val r = estabilizador.procesarFrame(td1Valido)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(true, r.vencido)
        assertEquals("Cédula de residencia (DIMEX) confirmado — DOCUMENTO VENCIDO", r.mensaje)
    }

    @Test
    fun reconoceComoTimUnaCedulaNacionalDeMenorPorLaFechaDeNacimiento() {
        // Mismo código de documento (IDCRI) que la cédula nacional de
        // adulto -- se distingue calculando la edad desde fechaNacimiento,
        // no por el código (ver LectorDocumentosIdentidad.reclasificarPorEdad).
        val td1Menor = """
            IDCRI2020202028<<<<<<<<<<<<<<<
            1806151M3001019CRI<<<<<<<<<<<8
            PEREZ<<CARLOS<ANDRES<<<<<<<<<<
        """.trimIndent()
        val hoyFijo = FechaDocumento(8, 9, 2026) // nacido 15/06/2018 -> 8 años
        val estabilizador = EstabilizadorLectura(obtenerFechaHoy = { hoyFijo })

        val r = estabilizador.procesarFrame(td1Menor)

        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(TipoDocumento.TARJETA_IDENTIDAD_MENOR, r.documento?.tipo)
        assertEquals("Tarjeta de Identidad de Menores confirmado", r.mensaje)
    }

    @Test
    fun sinFechaDeVencimientoNuncaSeMarcaComoVencido() {
        // Cédula nacional no trae fecha de vencimiento extraíble hoy.
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1)
        val r = estabilizador.procesarFrame("TRIBUNAL SUPREMO DE ELECCIONES\n1-1234-0567\nCOSTA RICA")
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(false, r.vencido)
    }

    @Test
    fun cedulaNacionalAcumulaNombreYApellidosVistosEnFramesDistintos() {
        // Hallazgo 2026-09-20 contra una cédula real: "Nombre:", "1°
        // Apellido:" y "2° Apellido:" son tres bloques separados en la
        // tarjeta (a diferencia de DIMEX, donde "Apellidos:" es un solo
        // bloque) -- rara vez ML Kit los lee los tres juntos en el mismo
        // frame. Un frame trae sólo el nombre, el siguiente sólo los
        // apellidos; el documento confirmado debe traer los tres, no sólo
        // lo que haya en el frame que completó el debounce.
        val estabilizador = EstabilizadorLectura(framesRequeridos = 2)
        val frameConNombre = """
            TRIBUNAL SUPREMO DE ELECCIONES
            1 2345 6789
            Nombre: JUAN CARLOS
        """.trimIndent()
        val frameConApellidos = """
            TRIBUNAL SUPREMO DE ELECCIONES
            1 2345 6789
            1°Apellido: GOMEZ
            2°Apellido: VARGAS
        """.trimIndent()

        estabilizador.procesarFrame(frameConNombre)
        val r = estabilizador.procesarFrame(frameConApellidos)

        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals("JUAN CARLOS", r.documento?.nombre)
        assertEquals("GOMEZ VARGAS", r.documento?.apellidos)
    }

    @Test
    fun cedulaNacionalLeidaSoloDelFrenteAvisaQueFaltaVoltearla() {
        // Sin MRZ en el texto (sólo lo que trae el frente con la foto): el
        // número se lee bien, pero nunca hay nombre desde esa cara -- ver
        // `LectorDocumentosIdentidad.leerDocumentoDeTexto`. El mensaje debe
        // decírselo a quien opera en vez de confirmar en silencio sin nombre
        // (bug reportado en pruebas reales, 2026-09-17).
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1)
        val r = estabilizador.procesarFrame("TRIBUNAL SUPREMO DE ELECCIONES\n1-1234-0567\nCOSTA RICA")
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(null, r.documento?.nombre)
        assertEquals("Ya tengo el número — muéstreme el reverso para el nombre", r.mensaje)
    }
}
