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

    /// Todos los acentos que puede haber en pantalla: los de cada sección y el
    /// de "Por correo" dentro de Externos.
    private fun acentos(oscuro: Boolean): List<Pair<String, AcentoSeccion>> =
        seccionesConAcento.map { it.name to acentoDeSeccion(it, oscuro)!! } +
            ("POR_CORREO" to acentoDeSeccion(SeccionPrincipal.EXTERNOS, oscuro, TipoExterno.POR_CORREO)!!)

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
            for ((nombre, acento) in acentos(oscuro)) {
                val razon = contraste(acento.color, acento.sobreColor)
                assertTrue("$nombre oscuro=$oscuro: texto sobre acento $razon", razon >= 4.5)
            }
        }
    }

    @Test
    fun elAcentoSeDistingueDelFondoEnAmbosTemas() {
        for (oscuro in listOf(false, true)) {
            for ((nombre, acento) in acentos(oscuro)) {
                for (fondo in fondos(oscuro)) {
                    val razon = contraste(acento.color, fondo)
                    assertTrue("$nombre oscuro=$oscuro: acento contra fondo $razon", razon >= 3.0)
                }
            }
        }
    }

    @Test
    fun cadaSeccionTieneUnColorDistinto() {
        for (oscuro in listOf(false, true)) {
            val colores = acentos(oscuro).map { it.second.color }
            assertEquals(colores.size, colores.toSet().size)
        }
    }

    @Test
    fun ningunAcentoCoincideConUnColorDeEstado() {
        val estados = listOf(ColorEscaneoConfirmado, ColorEscaneoInvalido, ColorEscaneoVencido)
        for (oscuro in listOf(false, true)) {
            for ((nombre, acento) in acentos(oscuro)) {
                estados.forEach { assertNotEquals("$nombre oscuro=$oscuro", it, acento.color) }
            }
        }
    }

    @Test
    fun porCorreoTieneSuPropioColorSoloDentroDeExternos() {
        for (oscuro in listOf(false, true)) {
            val proveedor = acentoDeSeccion(SeccionPrincipal.EXTERNOS, oscuro, TipoExterno.PROVEEDOR)!!.color
            val correo = acentoDeSeccion(SeccionPrincipal.EXTERNOS, oscuro, TipoExterno.POR_CORREO)!!.color
            assertNotEquals(proveedor, correo)
            // Sin tipo, o con Proveedor, Externos sigue en mango.
            assertEquals(proveedor, acentoDeSeccion(SeccionPrincipal.EXTERNOS, oscuro)!!.color)
            // En otra sección, el tipo de Externos no cuenta.
            assertEquals(
                acentoDeSeccion(SeccionPrincipal.ACTIVOS, oscuro)!!.color,
                acentoDeSeccion(SeccionPrincipal.ACTIVOS, oscuro, TipoExterno.POR_CORREO)!!.color,
            )
        }
    }

    @Test
    fun rutasConservaElAcentoDeLaPaleta() {
        assertNull(acentoDeSeccion(SeccionPrincipal.RUTAS, oscuro = false))
        assertNull(acentoDeSeccion(SeccionPrincipal.RUTAS, oscuro = true))
    }
}
