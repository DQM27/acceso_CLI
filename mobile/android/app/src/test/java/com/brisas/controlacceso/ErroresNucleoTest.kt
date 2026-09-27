package com.brisas.controlacceso

import kotlinx.coroutines.CancellationException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import uniffi.control_acceso_mobile.NucleoException

class ErroresNucleoTest {

    @Test
    fun convierteLosTresErroresEsperadosEnSuMensaje() {
        assertEquals("sin permiso", NucleoException.UsuarioInactivo("sin permiso").mensajeDeErrorEsperado())
        assertEquals(
            SecretoDispositivoNoEncontradoException().message,
            SecretoDispositivoNoEncontradoException().mensajeDeErrorEsperado(),
        )
        val almacen = SecretoDispositivoStoreException(IllegalStateException("x"))
        assertEquals(almacen.message, almacen.mensajeDeErrorEsperado())
    }

    @Test
    fun relanzaCualquierOtraExcepcionIncluidaLaCancelacion() {
        assertThrows(IllegalStateException::class.java) { IllegalStateException("bug").mensajeDeErrorEsperado() }
        assertThrows(CancellationException::class.java) { CancellationException("cancelada").mensajeDeErrorEsperado() }
    }
}
