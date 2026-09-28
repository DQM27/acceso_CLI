package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import uniffi.control_acceso_mobile.DatosPdf417Cedula

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
        val estabilizador = EstabilizadorLectura()
        val texto = "Un papel cualquiera sin ningun documento reconocible"
        // Los primeros frames no reconocibles siguen "buscando" (transición:
        // la mano moviéndose, un cartel de fondo); recién al 3ro seguido es
        // inválido.
        assertEquals(EstadoEscaneo.BUSCANDO, estabilizador.procesarFrame(texto).estado)
        assertEquals(EstadoEscaneo.BUSCANDO, estabilizador.procesarFrame(texto).estado)
        assertEquals(EstadoEscaneo.INVALIDO, estabilizador.procesarFrame(texto).estado)
    }

    @Test
    fun numeroDeNueveDigitosSinSenalesNoSeAceptaComoCedula() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 2, framesParaInvalido = 1)
        repeat(3) {
            val r = estabilizador.procesarFrame("Teléfono de contacto 888888888")
            assertEquals(EstadoEscaneo.INVALIDO, r.estado)
            assertNull(r.documento)
        }
    }

    /// El MRZ confirma en 2 frames coincidentes (número + nombres), no en
    /// 1: la línea de nombres no tiene dígito verificador.
    private fun EstabilizadorLectura.procesarDosVeces(texto: String): ResultadoEstabilizacion {
        procesarFrame(texto)
        return procesarFrame(texto)
    }

    @Test
    fun mrzValidoConfirmaEnDosFramesAunqueElFrenteExijaMas() {
        // Con checksum, número y fechas ya están probados; sólo se pide que
        // la lectura (nombres incluidos) se repita una vez.
        val estabilizador = EstabilizadorLectura(framesRequeridos = 3)
        val r1 = estabilizador.procesarFrame(td1Valido)
        assertEquals(EstadoEscaneo.BUSCANDO, r1.estado)
        val r2 = estabilizador.procesarFrame(td1Valido)
        assertEquals(EstadoEscaneo.CONFIRMADO, r2.estado)
        assertEquals("999888777", r2.documento?.numeroDocumento)
        assertEquals(FuenteDatos.MRZ, r2.documento?.fuenteDatos)
    }

    @Test
    fun mrzConNombreMalLeidoEnUnFrameNoSeConfirmaConEseNombre() {
        val nombreMalLeido = td1Valido.replace("PEREZ<<MARIA", "PEREZ<<MAPIA")
        val estabilizador = EstabilizadorLectura()
        assertEquals(EstadoEscaneo.BUSCANDO, estabilizador.procesarFrame(nombreMalLeido).estado)
        assertEquals(EstadoEscaneo.BUSCANDO, estabilizador.procesarFrame(td1Valido).estado)
        val r = estabilizador.procesarFrame(td1Valido)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals("MARIA JOSE", r.documento?.nombre)
    }

    @Test
    fun mrzConNombresQueNuncaCoincidenEnteroSeResuelvePorVotacionPorLetra() {
        // Ninguna lectura del nombre se repite entera en los 3 primeros
        // frames, pero cada letra sí es mayoría: la votación por carácter
        // confirma en el 3.º (antes hacía falta esperar al 4.º y a que
        // "MARIA" se repitiera completa).
        val estabilizador = EstabilizadorLectura()
        val variantes = listOf("MAPIA", "MARIA", "MARLA")
        val resultados = variantes.map { estabilizador.procesarFrame(td1Valido.replace("MARIA", it)) }
        assertEquals(EstadoEscaneo.BUSCANDO, resultados[1].estado)
        assertEquals(EstadoEscaneo.CONFIRMADO, resultados[2].estado)
        assertEquals("MARIA JOSE", resultados[2].documento?.nombre)
    }

    @Test
    fun mrzCorruptoNuncaConfirmaYNoSeMarcaComoInvalido() {
        // MRZ en cuadro pero mal leído = lectura parcial (foco/reflejo):
        // BUSCANDO, sin marco rojo ni vibración de error.
        val estabilizador = EstabilizadorLectura(framesRequeridos = 3)
        repeat(5) {
            val r = estabilizador.procesarFrame(td1Corrupto)
            assertEquals(EstadoEscaneo.BUSCANDO, r.estado)
            assertEquals("Leyendo el reverso — mantenga firme", r.mensaje)
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

        // No soportado es definitivo, no depende de otra lectura.
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
        assertEquals("Licencia de conducir — no lo mueva", r.mensaje)
    }

    @Test
    fun mensajeDeConfirmacionNombraElTipoDetectadoPorFrente() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1)
        val r = estabilizador.procesarFrame(licenciaTexto)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals("Listo: Licencia de conducir", r.mensaje)
    }

    @Test
    fun mensajeDeConfirmacionNombraElTipoDetectadoPorMrz() {
        val r = EstabilizadorLectura().procesarDosVeces(td1Valido)
        assertEquals("Listo: Cédula de residencia (DIMEX)", r.mensaje)
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
        assertEquals("Detecté Gafete de contratista — aquí va el documento del contratista", r.mensaje)
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
        val r = EstabilizadorLectura().procesarDosVeces(td1CedulaNacional)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(TipoDocumento.CEDULA_NACIONAL, r.documento?.tipo)
        assertEquals("Listo: Cédula de identidad", r.mensaje)
    }

    // --- Vigencia (fecha inyectada, no depende del reloj real) ---

    @Test
    fun documentoVigenteNoSeMarcaComoVencido() {
        val hoyFijo = FechaDocumento(1, 1, 2026) // antes del 03-04-2030 de licenciaTexto
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1, obtenerFechaHoy = { hoyFijo })
        val r = estabilizador.procesarFrame(licenciaTexto)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(false, r.vencido)
        assertEquals("Listo: Licencia de conducir", r.mensaje)
    }

    @Test
    fun documentoVencidoSeAnunciaEnElMensajeDeConfirmacionPorFrente() {
        val hoyFijo = FechaDocumento(1, 1, 2031) // después del 03-04-2030 de licenciaTexto
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1, obtenerFechaHoy = { hoyFijo })
        val r = estabilizador.procesarFrame(licenciaTexto)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(true, r.vencido)
        assertEquals("Listo: Licencia de conducir — VENCIDO el 03-04-2030", r.mensaje)
    }

    @Test
    fun documentoVencidoSeAnunciaEnElMensajeDeConfirmacionPorMrz() {
        // td1Valido vence 01/01/2030 -- fijamos "hoy" después de esa fecha.
        val hoyFijo = FechaDocumento(1, 1, 2031)
        val estabilizador = EstabilizadorLectura(obtenerFechaHoy = { hoyFijo })
        val r = estabilizador.procesarDosVeces(td1Valido)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(true, r.vencido)
        assertEquals("Listo: Cédula de residencia (DIMEX) — VENCIDO el 01-01-2030", r.mensaje)
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

        val r = estabilizador.procesarDosVeces(td1Menor)

        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(TipoDocumento.TARJETA_IDENTIDAD_MENOR, r.documento?.tipo)
        assertEquals("Listo: Tarjeta de Identidad de Menores", r.mensaje)
    }

    @Test
    fun sinFechaDeVencimientoNuncaSeMarcaComoVencido() {
        // Cédula nacional no trae fecha de vencimiento extraíble hoy.
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1)
        val r = estabilizador.procesarFrame("TRIBUNAL SUPREMO DE ELECCIONES\n1-1234-0567\nNombre: JUAN\nCOSTA RICA")
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

    private val frenteSinNombre = "TRIBUNAL SUPREMO DE ELECCIONES\n1-1234-0567\nCOSTA RICA"

    @Test
    fun cedulaNacionalLeidaSoloDelFrenteEsperaElReversoSinConfirmar() {
        // Antes confirmaba al instante con "muéstreme el reverso", pero
        // confirmar cierra la cámara: el reverso nunca se llegaba a leer.
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1, framesEsperaReverso = 5)
        val r = estabilizador.procesarFrame(frenteSinNombre)
        assertEquals(EstadoEscaneo.BUSCANDO, r.estado)
        assertNull(r.documento)
        assertEquals("Ya tengo el número — muéstreme la otra cara para el nombre", r.mensaje)
    }

    @Test
    fun alVoltearLaCedulaElMrzConfirmaConElNombre() {
        val td1CedulaNacional = """
            IDCRI1011101119<<<<<<<<<<<<<<<
            9001011F3001019CRI<<<<<<<<<<<8
            PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
        """.trimIndent()
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1, framesEsperaReverso = 20)
        estabilizador.procesarFrame(frenteSinNombre)
        estabilizador.procesarFrame("") // volteando la tarjeta
        val r = estabilizador.procesarDosVeces(td1CedulaNacional)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals(FuenteDatos.MRZ, r.documento?.fuenteDatos)
        assertEquals("MARIA JOSE", r.documento?.nombre)
    }

    @Test
    fun sinReversoConfirmaSoloElNumeroAlAgotarLaEspera() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1, framesEsperaReverso = 3)
        estabilizador.procesarFrame(frenteSinNombre)
        estabilizador.procesarFrame("")
        estabilizador.procesarFrame("")
        val r = estabilizador.procesarFrame("")
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals("112340567", r.documento?.numeroDocumento)
        assertNull(r.documento?.nombre)
        assertEquals("Listo: Cédula de identidad — sin nombre, complételo a mano", r.mensaje)
    }

    // --- Retroalimentación en cámara ---

    @Test
    fun informaAvanceHaciaLaConfirmacion() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 2)
        val r = estabilizador.procesarFrame("Licencia de Conducir\nNº: 112340567\nVencimiento 03-04-2099")
        assertEquals(EstadoEscaneo.BUSCANDO, r.estado)
        assertEquals(0.5f, r.progreso)
    }

    @Test
    fun sugiereQuitarElReflejoSiElTipoSeVePeroLosDatosNo() {
        val estabilizador = EstabilizadorLectura()
        var r = estabilizador.procesarFrame("REPUBLICA DE COSTA RICA\nLicencia de Conducir")
        assertEquals("Licencia de conducir — no lo mueva", r.mensaje)
        repeat(5) { r = estabilizador.procesarFrame("REPUBLICA DE COSTA RICA\nLicencia de Conducir") }
        assertEquals("Licencia de conducir — incline un poco para quitar el reflejo", r.mensaje)
    }

    // --- Votación por carácter (auditoría OCR 2026-09-28) ---

    private fun licencia(numero: String) = "Licencia de Conducir\nNº: $numero\nVencimiento 03-04-2030"

    @Test
    fun votacionConfirmaAunqueNingunFrameSeLeyeraEnteroBien() {
        // Cada frame erra en un dígito distinto (reflejo que se mueve).
        // Con igualdad de cadenas esto nunca confirmaba.
        val estabilizador = EstabilizadorLectura(framesRequeridos = 2)
        assertEquals(EstadoEscaneo.BUSCANDO, estabilizador.procesarFrame(licencia("712340567")).estado)
        assertEquals(EstadoEscaneo.BUSCANDO, estabilizador.procesarFrame(licencia("118340567")).estado)
        val r = estabilizador.procesarFrame(licencia("112340561"))
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals("112340567", r.documento?.numeroDocumento)
    }

    @Test
    fun modoGafeteNoConfirmaConLecturasQueSeContradicen() {
        // Confirmar un gafete dispara una salida real: dos "16" y un "18"
        // no alcanzan; hace falta un tercer "16".
        fun gafete(numero: String) = "CARNÉ\nPROVISIONAL\nCRC - $numero\nCONTRATISTAS\nCosta Rica"
        val estabilizador = EstabilizadorLectura(modo = ModoEscaneoDocumento.GAFETE_CONTRATISTA)
        estabilizador.procesarFrame(gafete("16"))
        estabilizador.procesarFrame(gafete("16"))
        assertEquals(EstadoEscaneo.BUSCANDO, estabilizador.procesarFrame(gafete("18")).estado)
        val r = estabilizador.procesarFrame(gafete("16"))
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals("16", r.documento?.numeroDocumento)
    }

    @Test
    fun framesMenosNitidosNecesitanMasRespaldo() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 2)
        estabilizador.procesarFrame(licencia("112340567"), peso = 0.5f)
        assertEquals(EstadoEscaneo.BUSCANDO, estabilizador.procesarFrame(licencia("112340567"), peso = 0.5f).estado)
        assertEquals(EstadoEscaneo.CONFIRMADO, estabilizador.procesarFrame(licencia("112340567"), peso = 0.5f).estado)
    }

    @Test
    fun otraPersonaNoHeredaLosCamposDeLaAnterior() {
        // Se cambia de cédula sin que cambie el tipo: el nombre de la
        // primera no puede terminar pegado al número de la segunda.
        // Sin espera del reverso, para que confirme con lo que haya.
        val estabilizador = EstabilizadorLectura(framesRequeridos = 2, framesEsperaReverso = 0)
        estabilizador.procesarFrame("TRIBUNAL SUPREMO DE ELECCIONES\n1 2345 6789\nNombre: JUAN CARLOS")
        estabilizador.procesarFrame("TRIBUNAL SUPREMO DE ELECCIONES\n4 9876 5432\n1°Apellido: GOMEZ")
        val r = estabilizador.procesarFrame("TRIBUNAL SUPREMO DE ELECCIONES\n4 9876 5432\n1°Apellido: GOMEZ")
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals("498765432", r.documento?.numeroDocumento)
        assertNull(r.documento?.nombre)
    }

    @Test
    fun reflejoMedidoSostenidoSeAvisaAunqueNoHayaPasadoElTiempo() {
        val estabilizador = EstabilizadorLectura()
        val conReflejo = CalidadFrame(nitidez = 30f, fraccionReflejo = 0.2f)
        var r = estabilizador.procesarFrame("REPUBLICA DE COSTA RICA\nLicencia de Conducir", calidad = conReflejo)
        assertEquals("Licencia de conducir — no lo mueva", r.mensaje)
        repeat(2) { r = estabilizador.procesarFrame("REPUBLICA DE COSTA RICA\nLicencia de Conducir", calidad = conReflejo) }
        assertEquals("Hay reflejo — incline un poco el documento", r.mensaje)
    }

    @Test
    fun ofreceLaOrientacionDelDocumentoParaElEncuadre() {
        val r = EstabilizadorLectura().procesarFrame(licenciaTexto)
        assertEquals(OrientacionEncuadre.HORIZONTAL, r.orientacionSugerida)
        val mrz = EstabilizadorLectura().procesarFrame(td1Valido)
        assertEquals(true, mrz.hayMrz)
    }

    // --- PDF417 de la cédula anterior ---

    private val datosPdf417 = DatosPdf417Cedula(cedula = "112340567", nombre = "JUAN CARLOS", apellidos = "PEREZ MORA")

    @Test
    fun pdf417ConfirmaEnUnaLecturaConCedulaYNombre() {
        val r = EstabilizadorLectura().procesarPdf417(datosPdf417)
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals("112340567", r.documento?.numeroDocumento)
        assertEquals("JUAN CARLOS", r.documento?.nombre)
        assertEquals("PEREZ MORA", r.documento?.apellidos)
        assertEquals(FuenteDatos.PDF417, r.documento?.fuenteDatos)
        assertEquals("Listo: Cédula de identidad", r.mensaje)
    }

    @Test
    fun pdf417EnModoGafeteNoConfirma() {
        val r = EstabilizadorLectura(modo = ModoEscaneoDocumento.GAFETE_CONTRATISTA).procesarPdf417(datosPdf417)
        assertEquals(EstadoEscaneo.BUSCANDO, r.estado)
        assertNull(r.documento)
    }

    @Test
    fun conVariosTextosUnoQueNoLeeCedeAlSiguienteDelMismoFrame() {
        val estabilizador = EstabilizadorLectura(framesRequeridos = 1)
        val r = estabilizador.procesarTextos(listOf("texto de fondo sin documento", licenciaTexto))
        assertEquals(EstadoEscaneo.CONFIRMADO, r.estado)
        assertEquals("112340567", r.documento?.numeroDocumento)
    }
}
