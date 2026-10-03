package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AvisoPushTest {
    @Test
    fun `usa titulo y cuerpo de los datos`() {
        val aviso = avisoDeMensaje(mapOf("titulo" to "Equipo retirado", "cuerpo" to "Este teléfono fue retirado"), null, null)
        assertEquals(AvisoPush("Equipo retirado", "Este teléfono fue retirado"), aviso)
    }

    @Test
    fun `los datos ganan sobre el bloque notification`() {
        val aviso = avisoDeMensaje(mapOf("titulo" to "De datos"), "De notification", "Cuerpo notification")
        assertEquals(AvisoPush("De datos", "Cuerpo notification"), aviso)
    }

    @Test
    fun `acepta un mensaje de prueba de la consola de Firebase`() {
        val aviso = avisoDeMensaje(emptyMap(), "Prueba", "Hola")
        assertEquals(AvisoPush("Prueba", "Hola"), aviso)
    }

    @Test
    fun `sin titulo usa el nombre de la app`() {
        assertEquals(AvisoPush("Lattis", "Sólo cuerpo"), avisoDeMensaje(mapOf("cuerpo" to "Sólo cuerpo"), null, null))
    }

    @Test
    fun `mensaje silencioso no muestra nada`() {
        assertNull(avisoDeMensaje(mapOf("tipo" to "sincronizar"), null, null))
        assertNull(avisoDeMensaje(mapOf("titulo" to "  ", "cuerpo" to ""), " ", null))
    }

    @Test
    fun `url del registro de token con o sin barra final`() {
        val esperado = "https://x.supabase.co/rest/v1/rpc/registrar_token_push"
        assertEquals(esperado, urlRegistroTokenPush("https://x.supabase.co"))
        assertEquals(esperado, urlRegistroTokenPush("https://x.supabase.co/"))
    }

    @Test
    fun `cuerpo del registro de token`() {
        assertEquals("{\"p_token\":\"abc:DEF-123_x\"}", cuerpoRegistroTokenPush("abc:DEF-123_x"))
        assertEquals("{\"p_token\":\"a\\\"b\\\\c\"}", cuerpoRegistroTokenPush("a\"b\\c"))
    }

    @Test
    fun `tipo emergente va por el canal de emergencias`() {
        assertEquals(
            AvisoPush("Evacuación", "Salir por el portón norte", emergente = true),
            avisoDeMensaje(mapOf("titulo" to "Evacuación", "cuerpo" to "Salir por el portón norte", "tipo" to "emergente"), null, null),
        )
        assertEquals(false, avisoDeMensaje(mapOf("titulo" to "Aviso", "tipo" to "normal"), null, null)?.emergente)
        assertEquals(false, avisoDeMensaje(mapOf("titulo" to "Aviso"), null, null)?.emergente)
    }
}
