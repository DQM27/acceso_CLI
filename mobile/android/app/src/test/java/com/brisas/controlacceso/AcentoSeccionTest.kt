package com.brisas.controlacceso

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.compositeOver
import androidx.compose.ui.graphics.luminance
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AcentoSeccionTest {

    private val seccionesConColor = listOf(
        SeccionPrincipal.ACTIVOS,
        SeccionPrincipal.KOF,
        SeccionPrincipal.PROVEEDORES,
        SeccionPrincipal.CORREO,
    )

    private fun colores(oscuro: Boolean): List<Pair<String, Color>> =
        seccionesConColor.map { it.name to colorDeSeccion(it, oscuro)!! }

    /// Razón de contraste WCAG entre dos colores opacos.
    private fun contraste(a: Color, b: Color): Double {
        val (claro, oscuro) = listOf(a.luminance(), b.luminance()).sortedDescending()
        return (claro + 0.05) / (oscuro + 0.05)
    }

    private fun paleta(oscuro: Boolean) = if (oscuro) PaletaNoche else PaletaDia

    /// Detrás de la pestaña elegida: la barra (superficie) con la píldora gris
    /// encima, con las mismas opacidades que `BarraSecciones`.
    private fun pildora(oscuro: Boolean): Color {
        val p = paleta(oscuro)
        return p.texto.copy(alpha = if (oscuro) 0.12f else 0.07f).compositeOver(p.tarjeta)
    }

    @Test
    fun elNombreDeLaPestanaSeLeeSobreSuPildora() {
        for (oscuro in listOf(false, true)) {
            for ((nombre, color) in colores(oscuro)) {
                val razon = contraste(color, pildora(oscuro))
                assertTrue("$nombre oscuro=$oscuro: pestaña $razon", razon >= 4.5)
            }
        }
    }

    @Test
    fun elColorDeSeccionSeLeeSobreLaBarraYElFondo() {
        for (oscuro in listOf(false, true)) {
            val p = paleta(oscuro)
            for ((nombre, color) in colores(oscuro)) {
                for (fondo in listOf(p.tarjeta, p.fondo)) {
                    val razon = contraste(color, fondo)
                    assertTrue("$nombre oscuro=$oscuro: contra $fondo $razon", razon >= 4.5)
                }
            }
        }
    }

    @Test
    fun cadaSeccionTieneUnColorDistinto() {
        for (oscuro in listOf(false, true)) {
            val lista = colores(oscuro).map { it.second }
            assertEquals(lista.size, lista.toSet().size)
        }
    }

    @Test
    fun ningunColorDeSeccionEsElDeLosBotonesNiUnoDeEstado() {
        for (oscuro in listOf(false, true)) {
            val p = paleta(oscuro)
            val reservados = listOf(
                p.acento, p.relleno, p.exito, p.error, p.advertencia, p.textoAdvertencia,
                ColorEscaneoConfirmado, ColorEscaneoInvalido, ColorEscaneoVencido,
            )
            for ((nombre, color) in colores(oscuro)) {
                reservados.forEach { assertNotEquals("$nombre oscuro=$oscuro", it, color) }
            }
        }
    }

    @Test
    fun lasSeccionesEnDesarrolloNoTienenColorPropio() {
        for (seccion in listOf(SeccionPrincipal.RUTAS, SeccionPrincipal.VISITAS)) {
            assertNull(colorDeSeccion(seccion, oscuro = false))
            assertNull(colorDeSeccion(seccion, oscuro = true))
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
