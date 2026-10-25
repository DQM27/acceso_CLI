package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.control_acceso_mobile.BloqueManual
import uniffi.control_acceso_mobile.buscarEnManual
import uniffi.control_acceso_mobile.indiceManual
import uniffi.control_acceso_mobile.temaManual

/// La lectura y la búsqueda del manual se prueban en Rust
/// (`mobile/rust-core/src/manual/tests.rs`). Acá, que lleguen bien por el
/// puente y lo poco que decide la pantalla: a qué bloque saltar.
class PantallaAyudaTest {
    @Test
    fun `el manual del telefono llega por el puente con su indice`() {
        val temas = indiceManual().flatMap { it.temas }
        assertTrue(temas.size > 10)
        assertTrue(temas.all { it.ruta.startsWith("movil/") })
        val tema = temaManual(temas.first().ruta)
        assertEquals(temas.first().titulo, tema?.titulo)
    }

    @Test
    fun `un resultado lleva a la seccion y la pantalla encuentra ese titulo`() {
        val resultado = buscarEnManual("Este encargado ya tiene un gafete provisional prestado").first()
        val tema = temaManual(resultado.ruta) ?: error("no existe ${resultado.ruta}")

        val indice = indiceDeAncla(tema.bloques, resultado.ancla)

        val titulo = tema.bloques[indice] as BloqueManual.Titulo
        assertEquals("Mensajes que pueden aparecer", titulo.tramos.joinToString("") { it.texto })
    }

    @Test
    fun `sin ancla o con una que no existe no salta`() {
        val bloques = temaManual(indiceManual().first().temas.first().ruta)!!.bloques
        assertEquals(-1, indiceDeAncla(bloques, null))
        assertEquals(-1, indiceDeAncla(bloques, "no-existe"))
    }
}
