package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LecturaFrameTest {

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

    // --- Métricas (sin datos personales) ---

    @Test
    fun metricasResumenMedianasYTiempoHastaConfirmar() {
        var ahora = 0L
        val registros = mutableListOf<String>()
        val metricas = MetricasOcr(habilitadas = true, reloj = { ahora }, registrar = { registros += it }, cadaCuantosFrames = 100)
        metricas.registrarFrame(nanosRecorte = 2_000_000, nanosReconocimiento = 80_000_000)
        ahora = 500_000_000
        metricas.registrarFrame(nanosRecorte = 4_000_000, nanosReconocimiento = 120_000_000)
        metricas.registrarDescarte()
        ahora = 1_500_000_000
        metricas.registrarConfirmacion()
        assertEquals(1, registros.size)
        assertTrue(registros[0], registros[0].startsWith("confirmado en 1500 ms"))
        assertTrue(registros[0], "descartados=1" in registros[0])
        assertTrue(registros[0], "reconocimiento_mediana_ms=100.0" in registros[0])
        // 2 frames terminados en 0,5 s: 1 intervalo -> 2 fps.
        assertTrue(registros[0], "fps=2.0" in registros[0])
    }

    @Test
    fun losDatosEstructuradosRepitenElResumen() {
        var ahora = 0L
        val metricas = MetricasOcr(habilitadas = true, reloj = { ahora }, cadaCuantosFrames = 100)
        metricas.registrarFrame(nanosRecorte = 2_000_000, nanosReconocimiento = 80_000_000)
        ahora = 500_000_000
        metricas.registrarFrame(nanosRecorte = 4_000_000, nanosReconocimiento = 120_000_000)
        metricas.registrarFallo()
        assertNull(metricas.datos()["ms_hasta_confirmar"])
        ahora = 1_500_000_000
        metricas.registrarConfirmacion()
        val datos = metricas.datos()
        assertEquals(1500L, datos["ms_hasta_confirmar"])
        assertEquals(2, datos["frames"])
        assertEquals(1, datos["fallos"])
        assertEquals(100f, datos["reconocimiento_mediana_ms"])
    }

    @Test
    fun soloLaPrimeraImagenDeCamaraMarcaElArranque() {
        var ahora = 0L
        val metricas = MetricasOcr(habilitadas = true, reloj = { ahora }, cadaCuantosFrames = 100)
        assertNull(metricas.datos()["ms_hasta_imagen_camara"])
        ahora = 650_000_000
        metricas.registrarImagenCamara()
        ahora = 900_000_000
        metricas.registrarImagenCamara()
        assertEquals(650L, metricas.datos()["ms_hasta_imagen_camara"])
    }

    @Test
    fun metricasDeshabilitadasNoRegistranNada() {
        val registros = mutableListOf<String>()
        val metricas = MetricasOcr(habilitadas = false, registrar = { registros += it }, cadaCuantosFrames = 1)
        metricas.registrarFrame(nanosRecorte = 1, nanosReconocimiento = 1)
        metricas.registrarConfirmacion()
        assertTrue(registros.isEmpty())
    }

    @Test
    fun registraCondicionesConfianzaYPlacasSinValores() {
        val metricas = MetricasOcr(habilitadas = true, cadaCuantosFrames = 100)
        metricas.registrarCalidad(CalidadFrame(nitidez = 10f, fraccionReflejo = 0.01f, luminancia = 40f))
        metricas.registrarCalidad(CalidadFrame(nitidez = 30f, fraccionReflejo = 0.05f, luminancia = 60f))
        metricas.registrarTexto(listOf(0.9f, 0.1f, 0.8f), umbralBajo = 0.25f, versionesTexto = 2)
        metricas.registrarTexto(listOf(0f, 0f), umbralBajo = 0.25f, versionesTexto = 1)
        val placa = extraerVehiculoConDetalle("E37 1931")
        metricas.registrarLecturaVehiculo(placa, indiceTexto = 0)
        metricas.registrarLecturaVehiculo(null, indiceTexto = null)
        metricas.registrarReinicioVotacion()
        val datos = metricas.datos()
        assertEquals(2, datos["frames_medidos"])
        assertEquals(40f, datos["luminancia_p10"])
        assertEquals(1, datos["frames_textos_distintos"])
        assertEquals(5, datos["palabras"])
        assertEquals(1, datos["palabras_confianza_baja"])
        assertEquals(1, datos["frames_sin_confianza"])
        assertEquals(mapOf("CARGA" to 1), datos["vehiculo_formatos"])
        assertEquals(1, datos["vehiculo_cl_restituida"])
        assertEquals(mapOf("visual" to 1), datos["vehiculo_por_version_texto"])
        assertEquals(1, datos["vehiculo_frames_sin_lectura"])
        assertEquals(1, datos["reinicios_votacion"])
        // Nunca el valor de la placa.
        assertFalse(datos.values.any { it.toString().contains("371931") })
    }

    @Test
    fun registraComoTerminaLaSesionDeDocumentoSinNumeroNiNombre() {
        var ahora = 0L
        val metricas = MetricasOcr(habilitadas = true, reloj = { ahora }, cadaCuantosFrames = 100)
        ahora = 300_000_000
        metricas.registrarResultadoDocumento(ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = "x"))
        ahora = 800_000_000
        metricas.registrarResultadoDocumento(ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = "x", progreso = 0.5f))
        ahora = 1_200_000_000
        val documento = DocumentoDetectado(TipoDocumento.CEDULA_NACIONAL, "112340567", nombre = "JUAN", fuenteDatos = FuenteDatos.MRZ)
        metricas.registrarResultadoDocumento(
            ResultadoEstabilizacion(EstadoEscaneo.CONFIRMADO, documento = documento, mensaje = "x", progreso = 1f, hayMrz = true),
        )
        val datos = metricas.datos()
        assertEquals(800L, datos["ms_hasta_primera_lectura"])
        assertEquals(mapOf("BUSCANDO" to 2, "CONFIRMADO" to 1), datos["documento_estados"])
        assertEquals(1, datos["documento_frames_con_mrz"])
        assertEquals("CEDULA_NACIONAL", datos["documento_confirmado_tipo"])
        assertEquals("MRZ", datos["documento_confirmado_fuente"])
        assertEquals(true, datos["documento_confirmado_con_nombre"])
        assertFalse(datos.values.any { it.toString().contains("112340567") || it.toString().contains("JUAN") })
    }

    @Test
    fun laPlacaSeMuestraComoVaImpresa() {
        assertEquals("CL 371931", placaComoSeImprime("CL371931"))
        assertEquals("BPH-485", placaComoSeImprime("BPH485"))
        assertEquals("M 947369", placaComoSeImprime("M947369"))
        assertEquals("22906", placaComoSeImprime("22906"))
    }
}
