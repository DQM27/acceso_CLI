package com.brisas.controlacceso

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PaletaLattisTest {

    private val paletas = listOf("día" to PaletaDia, "noche" to PaletaNoche)

    /// Razón de contraste WCAG entre dos colores opacos.
    private fun contraste(a: Color, b: Color): Double {
        val (claro, oscuro) = listOf(a.luminance(), b.luminance()).sortedDescending()
        return (claro + 0.05) / (oscuro + 0.05)
    }

    private fun exigir(nombre: String, a: Color, b: Color, minimo: Double) {
        val razon = contraste(a, b)
        assertTrue("$nombre: $razon < $minimo", razon >= minimo)
    }

    @Test
    fun losTextosSeLeenSobreElFondoYLaTarjeta() {
        for ((tema, p) in paletas) {
            for (superficie in listOf(p.fondo, p.tarjeta)) {
                exigir("$tema texto", p.texto, superficie, 4.5)
            }
            // El secundario y los de estado van sobre tarjetas y franjas; sobre
            // el fondo de Tokyo Day dan 4,2:1 y 4,35:1 (valores del
            // escritorio, que se quedan igual).
            exigir("$tema secundario", p.textoSecundario, p.tarjeta, 4.5)
            exigir("$tema error", p.error, p.tarjeta, 4.5)
            exigir("$tema advertencia", p.textoAdvertencia, p.tarjeta, 4.5)
        }
    }

    @Test
    fun losRellenosLlevanTextoLegible() {
        for ((tema, p) in paletas) {
            exigir("$tema botón general", p.relleno, p.sobreRelleno, 4.5)
            exigir("$tema aviso de éxito", p.rellenoExito, p.sobreRelleno, 4.5)
            exigir("$tema aviso de información", p.rellenoInfo, p.sobreRelleno, 4.5)
            exigir("$tema aviso de error", p.rellenoError, p.sobreRelleno, 4.5)
            exigir("$tema texto sobre acento", p.acento, p.sobreAcento, 4.5)
        }
    }

    @Test
    fun laTarjetaDeAdentroSeLeeYSeDistingue() {
        for ((tema, p) in paletas) {
            exigir("$tema texto adentro", p.texto, p.adentroSuave, 4.5)
            exigir("$tema secundario adentro", p.textoSecundario, p.adentroSuave, 4.5)
            exigir("$tema error adentro", p.error, p.adentroSuave, 4.5)
            exigir("$tema palabra ADENTRO", p.textoAdentro, p.adentroSuave, 4.5)
            // La franja, contra el fondo de la propia tarjeta (elemento
            // gráfico: 3:1).
            exigir("$tema franja adentro", p.franjaAdentro, p.adentroSuave, 3.0)
        }
        // Que no se confunda con una tarjeta normal.
        assertTrue(PaletaDia.adentroSuave != PaletaDia.tarjeta)
        assertTrue(PaletaNoche.adentroSuave != PaletaNoche.tarjeta)
    }

    @Test
    fun laTarjetaDeQuienNoPuedeEntrarSeLeeYSeDistingue() {
        for ((tema, p) in paletas) {
            exigir("$tema texto bloqueado", p.texto, p.bloqueadoSuave, 4.5)
            exigir("$tema secundario bloqueado", p.textoSecundario, p.bloqueadoSuave, 4.5)
            exigir("$tema palabra DENEGADO/VENCIDO", p.textoBloqueado, p.bloqueadoSuave, 4.5)
            exigir("$tema franja bloqueado", p.franjaBloqueado, p.bloqueadoSuave, 3.0)
        }
        for (p in listOf(PaletaDia, PaletaNoche)) {
            assertTrue(p.bloqueadoSuave != p.adentroSuave)
            assertTrue(p.bloqueadoSuave != p.salidaSuave)
            assertTrue(p.bloqueadoSuave != p.tarjeta)
        }
    }

    @Test
    fun laTarjetaDeSalidaPorGafeteSeLee() {
        for ((tema, p) in paletas) {
            exigir("$tema texto salida", p.texto, p.salidaSuave, 4.5)
            exigir("$tema secundario salida", p.textoSecundario, p.salidaSuave, 4.5)
            // La hora de otro día sale en ámbar.
            exigir("$tema hora de otro día salida", p.textoAdvertencia, p.salidaSuave, 4.5)
            exigir("$tema franja salida", p.franjaSalida, p.salidaSuave, 3.0)
        }
        assertTrue(PaletaDia.salidaSuave != PaletaDia.adentroSuave)
        assertTrue(PaletaNoche.salidaSuave != PaletaNoche.adentroSuave)
    }

    @Test
    fun enOscuroLaTarjetaSeSeparaDelFondo() {
        // Con Tokyo Night daba 1,05:1 y la lista parecía una sola mancha.
        exigir("tarjeta contra fondo", PaletaNoche.tarjeta, PaletaNoche.fondo, 1.15)
        exigir("realce contra tarjeta", PaletaNoche.realce, PaletaNoche.tarjeta, 1.15)
    }

    @Test
    fun elEsquemaDeMaterialSaleDeLaPaleta() {
        assertEquals(PaletaNoche.fondo, EsquemaNoche.background)
        assertEquals(PaletaNoche.tarjeta, EsquemaNoche.surface)
        assertEquals(PaletaNoche.acento, EsquemaNoche.primary)
        assertEquals(PaletaDia.texto, EsquemaDia.onSurface)
        assertEquals(PaletaDia.textoSecundario, EsquemaDia.onSurfaceVariant)
    }
}
