package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.control_acceso_mobile.MotivoPdf417

class LecturaFrameTest {

    // --- PDF417: sólo el prefijo cruza a Rust y todo queda en cero ---

    /// Clave pública del PDF417 de la cédula anterior (ver
    /// mobile/rust-core/src/pdf417_cedula.rs). Datos SINTÉTICOS.
    private val clave = intArrayOf(0x27, 0x30, 0x04, 0xA0, 0x00, 0x0F, 0x93, 0x12, 0xA0, 0xD1, 0x33, 0xE0, 0x03, 0xD0, 0x00, 0xDF, 0x00)

    private fun pdf417Sintetico(): ByteArray {
        fun campo(texto: String, largo: Int) = texto.toByteArray(Charsets.ISO_8859_1).copyOf(largo)
        val claro = campo("112340567", 9) + campo("PEREZ", 26) + campo("MORA", 26) + campo("JUAN", 30) +
            "M1990010120300101".toByteArray() + ByteArray(300) { 0x5A }
        return ByteArray(claro.size) { i -> (claro[i].toInt() xor clave[i % clave.size]).toByte() }
    }

    @Test
    fun extraeCedulaYNombreYPoneEnCeroLosBytesCrudos() {
        val crudo = pdf417Sintetico()
        val datos = extraerPdf417Cedula(crudo)
        assertEquals("112340567", datos?.cedula)
        assertEquals("JUAN", datos?.nombre)
        assertEquals("PEREZ MORA", datos?.apellidos)
        // Incluidas las "huellas" (la cola): nada queda en memoria.
        assertTrue(crudo.all { it == 0.toByte() })
    }

    @Test
    fun codigoCortoNoDaDatosYTambienSeBorra() {
        val crudo = pdf417Sintetico().copyOf(50)
        assertNull(extraerPdf417Cedula(crudo))
        assertTrue(crudo.all { it == 0.toByte() })
    }

    // --- Líneas con forma de MRZ ---

    @Test
    fun reconoceLineasDeMrzYDescartaTextoComun() {
        assertTrue(esLineaMrzProbable("IDCRI1000002190<C004780077<<<<"))
        assertTrue(esLineaMrzProbable("PEREZ<<MARIA<JOSE<<<<<<<<<<<<<"))
        assertTrue(esLineaMrzProbable("PEREZ «MARIA<JOSE<<<<<<<<<<<<<"))
        assertFalse(esLineaMrzProbable("TRIBUNAL SUPREMO DE ELECCIONES"))
        assertFalse(esLineaMrzProbable("<<"))
    }

    // --- Seguidor de la banda MRZ ---

    private val base = RegionGuiaOcr.TARJETA_ID

    private fun lineas(arriba: Float, abajo: Float) = listOf(
        FraccionesRect(0.05f, arriba, 0.95f, arriba + (abajo - arriba) / 3),
        FraccionesRect(0.05f, abajo - (abajo - arriba) / 3, 0.95f, abajo),
    )

    @Test
    fun sinMrzVistoSeLeeElRecuadroCompleto() {
        assertEquals(base, SeguidorBandaMrz().region(base))
    }

    @Test
    fun trasVerElMrzSeLeeSoloSuBandaConMargen() {
        val seguidor = SeguidorBandaMrz(margenVertical = 0.5f, margenHorizontal = 0f)
        seguidor.registrar(base, lineas(0.7f, 0.9f))
        val region = seguidor.region(base) as SubregionRecorte
        assertEquals(0.6f, region.fracciones.arriba, 1e-4f)
        assertEquals(1f, region.fracciones.abajo, 1e-4f)
    }

    @Test
    fun lasCajasLeidasDentroDeLaBandaSeLlevanAlRecuadro() {
        val seguidor = SeguidorBandaMrz(margenVertical = 0f, margenHorizontal = 0f)
        seguidor.registrar(base, lineas(0.5f, 1f))
        val banda = seguidor.region(base)
        // Dentro de la banda (0.5-1.0 del recuadro), el MRZ ocupa su mitad
        // inferior: en el recuadro es 0.75-1.0.
        seguidor.registrar(banda, lineas(0.5f, 1f))
        val region = seguidor.region(base) as SubregionRecorte
        assertEquals(0.75f, region.fracciones.arriba, 1e-4f)
    }

    @Test
    fun siLaBandaPierdeElMrzVuelveAlRecuadroCompleto() {
        val seguidor = SeguidorBandaMrz(fallosParaSoltar = 2)
        seguidor.registrar(base, lineas(0.7f, 0.9f))
        val banda = seguidor.region(base)
        seguidor.registrar(banda, emptyList())
        assertTrue(seguidor.region(base) is SubregionRecorte)
        seguidor.registrar(banda, emptyList())
        assertEquals(base, seguidor.region(base))
    }

    @Test
    fun laBandaNoSeAplicaAOtraOrientacionDelRecuadro() {
        val seguidor = SeguidorBandaMrz()
        seguidor.registrar(base, lineas(0.7f, 0.9f))
        assertEquals(RegionGuiaOcr.GAFETE_VERTICAL, seguidor.region(RegionGuiaOcr.GAFETE_VERTICAL))
    }

    @Test
    fun subregionRecortaDentroDelRecuadroGuia() {
        val completo = base.rectanguloEnPixeles(1000, 2000)
        val mitadInferior = SubregionRecorte(base, FraccionesRect(0f, 0.5f, 1f, 1f)).rectanguloEnPixeles(1000, 2000)
        assertEquals(completo.bottom, mitadInferior.bottom)
        assertEquals(completo.top + completo.height / 2, mitadInferior.top)
        assertEquals(completo.left, mitadInferior.left)
    }

    // --- Planificador: cuándo buscar el PDF417 ADEMÁS del texto ---

    @Test
    fun sinPdf417HabilitadoNuncaSeBuscaCodigo() {
        val planificador = PlanificadorLectores(habilitado = false)
        assertTrue((1..10).none { planificador.leerCodigo() })
    }

    @Test
    fun sinPistaBuscaCodigoCadaTantosFramesYConPistaEnTodos() {
        val planificador = PlanificadorLectores(habilitado = true, periodoSinPista = 3)
        assertEquals(3, (1..9).count { planificador.leerCodigo() })
        planificador.registrarTexto(pareceReversoConCodigo = true, hayMrz = false)
        assertTrue((1..5).all { planificador.leerCodigo() })
    }

    @Test
    fun laPistaSeOlvidaSiElReversoDejaDeVerse() {
        val planificador = PlanificadorLectores(habilitado = true, periodoSinPista = 100, framesMemoria = 2)
        planificador.registrarTexto(pareceReversoConCodigo = true, hayMrz = false)
        planificador.registrarTexto(pareceReversoConCodigo = false, hayMrz = false)
        assertTrue(planificador.leerCodigo())
        planificador.registrarTexto(pareceReversoConCodigo = false, hayMrz = false)
        assertFalse(planificador.leerCodigo())
    }

    @Test
    fun conMrzEnCuadroNoSeBuscaPdf417() {
        val planificador = PlanificadorLectores(habilitado = true, periodoSinPista = 1)
        planificador.registrarTexto(pareceReversoConCodigo = true, hayMrz = true)
        assertTrue((1..6).none { planificador.leerCodigo() })
    }

    // --- Métricas (sin datos personales) ---

    @Test
    fun metricasResumenMedianasYTiempoHastaConfirmar() {
        var ahora = 0L
        val registros = mutableListOf<String>()
        val metricas = MetricasOcr(habilitadas = true, reloj = { ahora }, registrar = { registros += it }, cadaCuantosFrames = 100)
        metricas.registrarFrame(conCodigo = false, nanosRecorte = 2_000_000, nanosReconocimiento = 80_000_000)
        ahora = 500_000_000
        metricas.registrarFrame(conCodigo = true, nanosRecorte = 4_000_000, nanosReconocimiento = 120_000_000)
        metricas.registrarDescarte()
        ahora = 1_500_000_000
        metricas.registrarConfirmacion()
        assertEquals(1, registros.size)
        assertTrue(registros[0], registros[0].startsWith("confirmado en 1500 ms"))
        assertTrue(registros[0], "descartados=1" in registros[0])
        assertTrue(registros[0], "reconocimiento_mediana_ms=100.0" in registros[0])
        // 2 frames terminados en 0,5 s: 1 intervalo -> 2 fps.
        assertTrue(registros[0], "fps=2.0" in registros[0])
        assertTrue(registros[0], "con_codigo=1" in registros[0])
    }

    @Test
    fun losDatosEstructuradosRepitenElResumen() {
        var ahora = 0L
        val metricas = MetricasOcr(habilitadas = true, reloj = { ahora }, cadaCuantosFrames = 100)
        metricas.registrarFrame(conCodigo = false, nanosRecorte = 2_000_000, nanosReconocimiento = 80_000_000)
        ahora = 500_000_000
        metricas.registrarFrame(conCodigo = true, nanosRecorte = 4_000_000, nanosReconocimiento = 120_000_000)
        metricas.registrarFallo()
        assertNull(metricas.datos()["ms_hasta_confirmar"])
        ahora = 1_500_000_000
        metricas.registrarConfirmacion()
        val datos = metricas.datos()
        assertEquals(1500L, datos["ms_hasta_confirmar"])
        assertEquals(2, datos["frames"])
        assertEquals(1, datos["frames_con_codigo"])
        assertEquals(1, datos["fallos"])
        assertEquals(100f, datos["reconocimiento_mediana_ms"])
    }

    @Test
    fun metricasDeshabilitadasNoRegistranNada() {
        val registros = mutableListOf<String>()
        val metricas = MetricasOcr(habilitadas = false, registrar = { registros += it }, cadaCuantosFrames = 1)
        metricas.registrarFrame(conCodigo = false, nanosRecorte = 1, nanosReconocimiento = 1)
        metricas.registrarConfirmacion()
        assertTrue(registros.isEmpty())
    }

    @Test
    fun elDiagnosticoDelPdf417CuentaDondeSePierdeLaLectura() {
        val metricas = MetricasOcr(habilitadas = true, cadaCuantosFrames = 100)
        // Frame sin códigos (lo esperable con poca resolución).
        metricas.registrarBusquedaCodigo(DiagnosticoCodigos(0, 0, emptyList(), emptyList(), emptyList(), anchoImagenPx = 900))
        // Frame con un código sin bytes y otro rechazado por el núcleo.
        metricas.registrarBusquedaCodigo(
            DiagnosticoCodigos(2, 1, listOf(MotivoPdf417.CEDULA_INVALIDA), listOf(512), listOf(410, 420), anchoImagenPx = 1100),
        )
        metricas.registrarErrorLectorCodigo()
        val datos = metricas.datos()
        assertEquals(1, datos["pdf417_frames_con_codigo_detectado"])
        assertEquals(2, datos["pdf417_codigos_detectados"])
        assertEquals(1, datos["pdf417_sin_bytes"])
        assertEquals(1, datos["pdf417_errores_lector"])
        assertEquals(mapOf("CEDULA_INVALIDA" to 1), datos["pdf417_motivos"])
        assertEquals(512f, datos["pdf417_bytes_max"])
        assertEquals(415f, datos["pdf417_ancho_codigo_px_mediana"])
        assertEquals(1000f, datos["imagen_ancho_px_mediana_con_codigo"])
    }

    @Test
    fun unPdf417CortoSeRechazaConMotivoYSeBorra() {
        val crudo = ByteArray(10) { 7 }
        assertEquals(MotivoPdf417.PREFIJO_CORTO, leerPdf417ConMotivo(crudo).motivo)
        assertTrue(crudo.all { it == 0.toByte() })
    }
}
