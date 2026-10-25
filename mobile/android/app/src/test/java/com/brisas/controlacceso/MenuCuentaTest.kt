package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
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

    @Test
    fun `sin version confirmada el item busca`() {
        val opcion = opcionActualizacion(EstadoActualizacion.SinAviso, EstadoBusqueda.INACTIVA)
        assertEquals("Buscar actualizaciones", opcion.texto)
        assertEquals(AccionActualizacion.BUSCAR, opcion.accion)
        assertEquals(false, opcion.resaltada)
    }

    @Test
    fun `mientras busca no se puede tocar`() {
        val opcion = opcionActualizacion(EstadoActualizacion.SinAviso, EstadoBusqueda.BUSCANDO)
        assertEquals("Buscando…", opcion.texto)
        assertEquals(AccionActualizacion.NINGUNA, opcion.accion)
    }

    @Test
    fun `tras avisar el resultado el item vuelve a ofrecer buscar`() {
        listOf(EstadoBusqueda.AL_DIA, EstadoBusqueda.SIN_RESPUESTA).forEach { busqueda ->
            assertEquals(
                AccionActualizacion.BUSCAR,
                opcionActualizacion(EstadoActualizacion.SinAviso, busqueda).accion,
            )
        }
    }

    @Test
    fun `cada resultado de la busqueda tiene su aviso corto`() {
        assertEquals("Está al día", avisoDeBusqueda(EstadoBusqueda.AL_DIA))
        assertEquals("No se pudo revisar", avisoDeBusqueda(EstadoBusqueda.SIN_RESPUESTA))
        assertNull(avisoDeBusqueda(EstadoBusqueda.INACTIVA))
        assertNull(avisoDeBusqueda(EstadoBusqueda.BUSCANDO))
    }
}
