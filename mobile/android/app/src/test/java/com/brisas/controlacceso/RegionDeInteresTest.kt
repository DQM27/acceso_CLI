package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test

class RegionDeInteresTest {

    @Test
    fun rotacion90IntercambiaAnchoYAlto() {
        assertEquals(1080 to 1920, dimensionesUpright(ancho = 1920, alto = 1080, rotacionGrados = 90))
    }

    @Test
    fun rotacion270IntercambiaAnchoYAlto() {
        assertEquals(1080 to 1920, dimensionesUpright(ancho = 1920, alto = 1080, rotacionGrados = 270))
    }

    @Test
    fun rotacion0NoCambiaNada() {
        assertEquals(1920 to 1080, dimensionesUpright(ancho = 1920, alto = 1080, rotacionGrados = 0))
    }

    @Test
    fun rotacion180NoCambiaNada() {
        assertEquals(1920 to 1080, dimensionesUpright(ancho = 1920, alto = 1080, rotacionGrados = 180))
    }

    @Test
    fun bloqueDentroDelAreaGuiaSeConserva() {
        // Imagen 1000x1000 -- área guía centrada, ~820x517.
        val bloques = listOf(BloqueTextoOcr(CajaTexto(400, 400, 600, 450), "DENTRO"))
        val resultado = filtrarTextoEnAreaGuia(bloques, anchoImagen = 1000, altoImagen = 1000)
        assertEquals("DENTRO", resultado)
    }

    @Test
    fun bloqueFueraDelAreaGuiaSeDescarta() {
        // Esquina superior izquierda, fuera del área guía centrada.
        val bloques = listOf(BloqueTextoOcr(CajaTexto(0, 0, 50, 20), "FONDO"))
        val resultado = filtrarTextoEnAreaGuia(bloques, anchoImagen = 1000, altoImagen = 1000)
        assertEquals("", resultado)
    }

    @Test
    fun bloqueParcialmenteDentroSeConserva() {
        // Se superpone apenas con el borde del área guía -- criterio
        // inclusivo (intersección, no contención total) para no perder
        // texto que quedó levemente cortado por el encuadre.
        val bloques = listOf(BloqueTextoOcr(CajaTexto(80, 400, 130, 450), "BORDE"))
        val resultado = filtrarTextoEnAreaGuia(bloques, anchoImagen = 1000, altoImagen = 1000)
        assertEquals("BORDE", resultado)
    }

    @Test
    fun mezclaDeBloquesSoloConservaLosDelAreaGuia() {
        val bloques = listOf(
            BloqueTextoOcr(CajaTexto(0, 0, 50, 20), "FONDO_IZQUIERDA"),
            BloqueTextoOcr(CajaTexto(400, 400, 600, 450), "DOCUMENTO_1"),
            BloqueTextoOcr(CajaTexto(950, 950, 1000, 1000), "FONDO_DERECHA"),
            BloqueTextoOcr(CajaTexto(400, 460, 600, 500), "DOCUMENTO_2"),
        )
        val resultado = filtrarTextoEnAreaGuia(bloques, anchoImagen = 1000, altoImagen = 1000)
        assertEquals("DOCUMENTO_1\nDOCUMENTO_2", resultado)
    }

    @Test
    fun bloqueSinCajaNuncaSeDescarta() {
        val bloques = listOf(BloqueTextoOcr(caja = null, texto = "SIN_CAJA"))
        val resultado = filtrarTextoEnAreaGuia(bloques, anchoImagen = 1000, altoImagen = 1000)
        assertEquals("SIN_CAJA", resultado)
    }
}
