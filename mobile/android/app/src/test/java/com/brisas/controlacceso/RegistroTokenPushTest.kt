package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test

/// Cómo se lee un aviso que llega (título de respaldo, emergente, id y hora
/// de envío) se prueba en el núcleo: mobile/rust-core/src/avisos.rs.
class RegistroTokenPushTest {
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
}
