package com.brisas.controlacceso

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AcentoSeccionTest {

    private val seccionesConAcento = listOf(
        SeccionPrincipal.ACTIVOS,
        SeccionPrincipal.KOF,
        SeccionPrincipal.PROVEEDORES,
        SeccionPrincipal.CORREO,
    )

    /// Todos los acentos que puede haber en pantalla: uno por sección.
    private fun acentos(oscuro: Boolean): List<Pair<String, AcentoSeccion>> =
        seccionesConAcento.map { it.name to acentoDeSeccion(it, oscuro)!! }

    /// Razón de contraste WCAG entre dos colores opacos.
    private fun contraste(a: Color, b: Color): Double {
        val (claro, oscuro) = listOf(a.luminance(), b.luminance()).sortedDescending()
        return (claro + 0.05) / (oscuro + 0.05)
    }

    private fun fondos(oscuro: Boolean): List<Color> =
        if (oscuro) listOf(EsquemaNoche.background, EsquemaNoche.surface) else listOf(EsquemaDia.background, EsquemaDia.surface)

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
    fun elTextoSobreElRellenoDeLosBotonesSeLeeEnAmbosTemas() {
        for (oscuro in listOf(false, true)) {
            for ((nombre, acento) in acentos(oscuro)) {
                val razon = contraste(acento.relleno, acento.sobreRelleno)
                assertTrue("$nombre oscuro=$oscuro: texto sobre relleno $razon", razon >= 4.5)
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
    fun lasSeccionesEnDesarrolloConservanElAcentoDeLaPaleta() {
        for (seccion in listOf(SeccionPrincipal.RUTAS, SeccionPrincipal.VISITAS)) {
            assertNull(acentoDeSeccion(seccion, oscuro = false))
            assertNull(acentoDeSeccion(seccion, oscuro = true))
        }
    }

    @Test
    fun laBarraMuestraCuatroSeccionesPorUso() {
        val visibles = SeccionPrincipal.entries.filterNot { it.enDesarrollo }
        assertEquals(
            listOf(SeccionPrincipal.ACTIVOS, SeccionPrincipal.PROVEEDORES, SeccionPrincipal.CORREO, SeccionPrincipal.KOF),
            visibles,
        )
    }
}
