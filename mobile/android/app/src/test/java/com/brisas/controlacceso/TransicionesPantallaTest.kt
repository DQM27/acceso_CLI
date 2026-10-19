package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotSame
import org.junit.Assert.assertSame
import org.junit.Test

class TransicionesPantallaTest {

    @Test
    fun losEscanerosTapanAlFormularioYEsteALaLista() {
        assertEquals(PanelConFormulario.LISTA, panelConFormulario(false, false, false))
        assertEquals(PanelConFormulario.FORMULARIO, panelConFormulario(true, false, false))
        assertEquals(PanelConFormulario.ESCANER_CEDULA, panelConFormulario(true, true, false))
        assertEquals(PanelConFormulario.ESCANER_PLACA, panelConFormulario(true, false, true))
        // Los dos escáneres a la vez (no debería pasar): gana el de cédula, como antes.
        assertEquals(PanelConFormulario.ESCANER_CEDULA, panelConFormulario(false, true, true))
    }

    @Test
    fun soloLosEscanerosSonDeCamara() {
        assertEquals(
            setOf(PanelConFormulario.ESCANER_CEDULA, PanelConFormulario.ESCANER_PLACA),
            PanelConFormulario.entries.filter { it.conCamara }.toSet(),
        )
    }

    @Test
    fun conCamaraEnJuegoElCambioEsInmediato() {
        for (camara in listOf(PanelConFormulario.ESCANER_CEDULA, PanelConFormulario.ESCANER_PLACA)) {
            for (otro in PanelConFormulario.entries) {
                assertSame(SinTransicion, transicionDePanel(camara, otro))
                assertSame(SinTransicion, transicionDePanel(otro, camara))
            }
        }
    }

    @Test
    fun listaYFormularioSeAnimanEntreSi() {
        assertNotSame(SinTransicion, transicionDePanel(PanelConFormulario.LISTA, PanelConFormulario.FORMULARIO))
        assertNotSame(SinTransicion, transicionDePanel(PanelConFormulario.FORMULARIO, PanelConFormulario.LISTA))
    }

    @Test
    fun cambiarDePestanaAlMismoLugarNoAnima() {
        assertSame(SinTransicion, transicionDePestana(1, 1))
        assertNotSame(SinTransicion, transicionDePestana(0, 2))
    }
}
