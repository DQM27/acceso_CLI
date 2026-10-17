package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.control_acceso_mobile.RolUsuario

class MenuCuentaTest {
    @Test
    fun `la inicial es la primera letra en mayuscula`() {
        assertEquals("D", inicialDe("daniel quintana"))
        assertEquals("É", inicialDe("  Élida Mora"))
    }

    @Test
    fun `un nombre vacio no deja el boton sin nada`() {
        assertEquals("?", inicialDe(""))
        assertEquals("?", inicialDe("   "))
    }

    @Test
    fun `cada rol tiene su etiqueta`() {
        assertEquals("Root", etiquetaRol(RolUsuario.ROOT))
        assertEquals("Administrador", etiquetaRol(RolUsuario.ADMINISTRADOR))
        assertEquals("Operador", etiquetaRol(RolUsuario.OPERADOR))
    }
}
