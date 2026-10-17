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

    @Test
    fun `sin version confirmada el item busca`() {
        val opcion = opcionActualizacion(EstadoActualizacion.SinAviso, EstadoBusqueda.INACTIVA)
        assertEquals("Buscar actualizaciones", opcion.texto)
        assertEquals(AccionActualizacion.BUSCAR, opcion.accion)
        assertEquals(false, opcion.resaltada)
    }

    @Test
    fun `mientras busca y cuando esta al dia no se puede tocar`() {
        assertEquals(
            AccionActualizacion.NINGUNA,
            opcionActualizacion(EstadoActualizacion.SinAviso, EstadoBusqueda.BUSCANDO).accion,
        )
        val alDia = opcionActualizacion(EstadoActualizacion.SinAviso, EstadoBusqueda.AL_DIA)
        assertEquals("Estás al día", alDia.texto)
        assertEquals(AccionActualizacion.NINGUNA, alDia.accion)
    }

    @Test
    fun `si no se pudo revisar deja reintentar`() {
        val opcion = opcionActualizacion(EstadoActualizacion.SinAviso, EstadoBusqueda.SIN_RESPUESTA)
        assertEquals(AccionActualizacion.BUSCAR, opcion.accion)
    }
}
