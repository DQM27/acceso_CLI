package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class EncuadreAdaptativoTest {

    private val H = OrientacionEncuadre.HORIZONTAL
    private val V = OrientacionEncuadre.VERTICAL

    private fun controlador(framesSinDocumentoParaVolver: Int = 3) = ControladorEncuadre(
        regionHorizontal = RegionGuiaOcr.TARJETA_ID,
        regionVertical = RegionGuiaOcr.GAFETE_VERTICAL,
        orientacionDelTexto = ::orientacionDeTextoDocumento,
        framesSinDocumentoParaVolver = framesSinDocumentoParaVolver,
    )

    private val praind = "Nombre: Ana Rojas\nNo. de cédula: 701000000\nFecha de inducción: 03/08/2026"
    private val cedula = "TRIBUNAL SUPREMO DE ELECCIONES\n1 2345 6789\nNombre: JUAN"

    @Test
    fun arrancaHorizontal() {
        assertEquals(H, controlador().orientacion)
    }

    @Test
    fun giraAVerticalAlLeerDosVecesUnDocumentoVertical() {
        val c = controlador()
        assertFalse(c.registrarTexto(praind, H))
        assertTrue(c.registrarTexto(praind, H))
        assertEquals(V, c.orientacion)
        assertEquals(RegionGuiaOcr.GAFETE_VERTICAL, c.regionParaFrame().first)
    }

    @Test
    fun unaCedulaNuncaGira() {
        val c = controlador()
        repeat(5) { assertFalse(c.registrarTexto(cedula, H)) }
        assertEquals(H, c.orientacion)
    }

    @Test
    fun sinDocumentoSondeaLaOtraRegionUnSoloFrame() {
        val c = controlador()
        c.registrarTexto("texto cualquiera sin documento", H)
        assertEquals(V, c.regionParaFrame().second) // sonda
        assertEquals(H, c.regionParaFrame().second) // vuelve a la actual
        assertEquals(H, c.orientacion) // la sonda no gira el recuadro
    }

    @Test
    fun laSondaQueReconoceUnGafeteVerticalHaceGirar() {
        val c = controlador()
        c.registrarTexto("nada", H)
        c.registrarTexto(praind, c.regionParaFrame().second)
        c.registrarTexto("nada", H)
        assertTrue(c.registrarTexto(praind, c.regionParaFrame().second))
        assertEquals(V, c.orientacion)
    }

    @Test
    fun vuelveAHorizontalSiElDocumentoVerticalSeRetira() {
        val c = controlador(framesSinDocumentoParaVolver = 3)
        c.registrarTexto(praind, H)
        c.registrarTexto(praind, H)
        assertEquals(V, c.orientacion)
        assertFalse(c.registrarTexto("", V))
        assertFalse(c.registrarTexto("", V))
        assertTrue(c.registrarTexto("", V))
        assertEquals(H, c.orientacion)
    }

    @Test
    fun enVerticalUnaCedulaLoDevuelveAHorizontal() {
        val c = controlador()
        c.registrarTexto(praind, H)
        c.registrarTexto(praind, H)
        c.registrarTexto(cedula, V)
        assertTrue(c.registrarTexto(cedula, V))
        assertEquals(H, c.orientacion)
    }

    @Test
    fun elMrzDelReversoCuentaComoHorizontal() {
        assertEquals(H, orientacionDeTextoDocumento("IDCRI1000002190<C004780077<<<<"))
    }
}
