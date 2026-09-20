package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test

class DetectorTextoNoReconocidoTest {

    private fun detector(esTipoEsperado: (String) -> Boolean) =
        DetectorTextoNoReconocido(esTipoEsperado = esTipoEsperado, framesRequeridos = 3, ventana = 5)

    @Test
    fun unSoloFrameMalLeidoNoMarcaInvalido() {
        // Mismo texto largo real, pero un solo frame no matchea -- glare o
        // ángulo puntual, el documento correcto sigue en cuadro.
        val det = detector { it == "RUTA CORRECTA" }
        assertEquals(false, det.procesarFrame("texto random largo que no matchea"))
        assertEquals(false, det.procesarFrame("RUTA CORRECTA"))
    }

    @Test
    fun variosFramesSeguidosSinMatchSiMarcanInvalido() {
        val det = detector { it == "RUTA CORRECTA" }
        det.procesarFrame("otro documento cualquiera con texto largo")
        det.procesarFrame("otro documento cualquiera con texto largo")
        val resultado = det.procesarFrame("otro documento cualquiera con texto largo")
        assertEquals(true, resultado)
    }

    @Test
    fun textoCortoNiSumaNiRompeLaRacha() {
        val det = detector { false }
        det.procesarFrame("sin coincidencia numero uno largo")
        det.procesarFrame("sin coincidencia numero dos largo")
        // Frame casi vacío (borroso/en blanco) en el medio -- no debe
        // reiniciar el conteo ya acumulado.
        det.procesarFrame("corto")
        val resultado = det.procesarFrame("sin coincidencia numero tres largo")
        assertEquals(true, resultado)
    }

    @Test
    fun frasesSeguidasQueSiCoincidenSacanLosFramesMalosDeLaVentana() {
        // Ventana de 5: dos frames sin match seguidos por tres que sí
        // matchean terminan corriendo los dos malos fuera de la ventana --
        // ya no alcanzan para el mínimo de 3.
        val det = detector { it == "OK" }
        det.procesarFrame("no coincide numero uno con texto largo")
        det.procesarFrame("no coincide numero dos con texto largo")
        det.procesarFrame("OK")
        det.procesarFrame("OK")
        val resultado = det.procesarFrame("OK")
        assertEquals(false, resultado)
    }
}
