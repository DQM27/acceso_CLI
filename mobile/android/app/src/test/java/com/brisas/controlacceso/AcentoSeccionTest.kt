package com.brisas.controlacceso

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AcentoSeccionTest {

    private val seccionesConAcento = listOf(SeccionPrincipal.ACTIVOS, SeccionPrincipal.KOF, SeccionPrincipal.EXTERNOS)

    /// Razón de contraste WCAG entre dos colores opacos.
    private fun contraste(a: Color, b: Color): Double {
        val (claro, oscuro) = listOf(a.luminance(), b.luminance()).sortedDescending()
        return (claro + 0.05) / (oscuro + 0.05)
    }

    private fun fondos(oscuro: Boolean): List<Color> =
        if (oscuro) listOf(TokyoNight.background, TokyoNight.surface) else listOf(TokyoDay.background, TokyoDay.surface)

    @Test
    fun elTextoSobreElAcentoSeLeeEnAmbosTemas() {
        for (oscuro in listOf(false, true)) {
            for (seccion in seccionesConAcento) {
                val acento = acentoDeSeccion(seccion, oscuro)!!
                val razon = contraste(acento.color, acento.sobreColor)
                assertTrue("$seccion oscuro=$oscuro: texto sobre acento $razon", razon >= 4.5)
            }
        }
    }

    @Test
    fun elAcentoSeDistingueDelFondoEnAmbosTemas() {
        for (oscuro in listOf(false, true)) {
            for (seccion in seccionesConAcento) {
                val acento = acentoDeSeccion(seccion, oscuro)!!
                for (fondo in fondos(oscuro)) {
                    val razon = contraste(acento.color, fondo)
                    assertTrue("$seccion oscuro=$oscuro: acento contra fondo $razon", razon >= 3.0)
                }
            }
        }
    }

    @Test
    fun cadaSeccionTieneUnColorDistinto() {
        for (oscuro in listOf(false, true)) {
            val colores = seccionesConAcento.map { acentoDeSeccion(it, oscuro)!!.color }
            assertEquals(colores.size, colores.toSet().size)
        }
    }

    @Test
    fun ningunAcentoCoincideConUnColorDeEstado() {
        val estados = listOf(ColorEscaneoConfirmado, ColorEscaneoInvalido, ColorEscaneoVencido)
        for (oscuro in listOf(false, true)) {
            for (seccion in seccionesConAcento) {
                val color = acentoDeSeccion(seccion, oscuro)!!.color
                estados.forEach { assertNotEquals("$seccion oscuro=$oscuro", it, color) }
            }
        }
    }

    @Test
    fun rutasConservaElAcentoDeLaPaleta() {
        assertNull(acentoDeSeccion(SeccionPrincipal.RUTAS, oscuro = false))
        assertNull(acentoDeSeccion(SeccionPrincipal.RUTAS, oscuro = true))
    }
}
