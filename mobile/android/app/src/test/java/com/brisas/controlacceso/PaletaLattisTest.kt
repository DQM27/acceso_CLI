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
            exigir("$tema aviso de error", p.rellenoError, p.sobreRelleno, 4.5)
            exigir("$tema texto sobre acento", p.acento, p.sobreAcento, 4.5)
        }
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
            assertTrue(p.bloqueadoSuave != p.salidaSuave)
            assertTrue(p.bloqueadoSuave != p.tarjeta)
        }
    }

    @Test
    fun laTarjetaDeSalidaSeLee() {
        // La búsqueda por gafete y quien ya está adentro en la de
        // contratistas: la palabra ADENTRO y su flecha van en el acento, y el
        // aviso de acceso del núcleo en rojo.
        for ((tema, p) in paletas) {
            exigir("$tema texto salida", p.texto, p.salidaSuave, 4.5)
            exigir("$tema secundario salida", p.textoSecundario, p.salidaSuave, 4.5)
            // La hora de otro día sale en ámbar.
            exigir("$tema hora de otro día salida", p.textoAdvertencia, p.salidaSuave, 4.5)
            exigir("$tema palabra ADENTRO", p.acento, p.salidaSuave, 4.5)
            exigir("$tema aviso de acceso", p.error, p.salidaSuave, 4.5)
            exigir("$tema franja salida", p.franjaSalida, p.salidaSuave, 3.0)
            assertTrue("$tema salida distinta de la tarjeta", p.salidaSuave != p.tarjeta)
        }
    }

    @Test
    fun laAdvertenciaSeLeeYNoSeConfundeConElError() {
        for ((tema, p) in paletas) {
            exigir("$tema franja de advertencia", p.textoAdvertencia, p.advertenciaSuave, 4.5)
            exigir("$tema aviso breve de advertencia", p.rellenoAdvertencia, p.sobreRelleno, 4.5)
            assertTrue("$tema advertencia distinta del error", p.advertenciaSuave != p.errorSuave)
            assertTrue("$tema relleno de advertencia distinto del de error", p.rellenoAdvertencia != p.rellenoError)
        }
    }

    @Test
    fun losCamposYElGafeteSeLeen() {
        // El número de gafete va en texto principal sobre `variante`, y los
        // textos de ayuda de los campos en secundario.
        for ((tema, p) in paletas) {
            exigir("$tema gafete", p.texto, p.variante, 7.0)
            exigir("$tema secundario sobre campo", p.textoSecundario, p.variante, 4.5)
            exigir("$tema secundario sobre fondo", p.textoSecundario, p.fondo, 4.5)
        }
    }

    @Test
    fun enClaroLaTarjetaSeSeparaDelFondo() {
        // Con Tokyo Day el fondo y la tarjeta daban 1,13:1; tarjeta blanca
        // sobre gris muy claro más el borde suave las separa.
        assertTrue(PaletaDia.tarjeta != PaletaDia.fondo)
        exigir("borde suave contra tarjeta", PaletaDia.bordeSuave, PaletaDia.tarjeta, 1.4)
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
