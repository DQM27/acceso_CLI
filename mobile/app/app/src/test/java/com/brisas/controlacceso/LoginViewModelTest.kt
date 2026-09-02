package com.brisas.controlacceso

import java.io.File
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.Nucleo

/// `LoginViewModel` no usa corrutinas (autenticar es síncrono, ver su
/// propio doc-comment) — estos tests no necesitan `runTest`/dispatchers de
/// prueba, sólo llamar y afirmar directo.
class LoginViewModelTest {
    private lateinit var archivo: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        archivo = File.createTempFile("login_test", ".db").apply { deleteOnExit() }
        nucleo = NucleoDePrueba.abrir(archivo, NucleoDePrueba.sqlUsuarioRoot())
    }

    @After
    fun limpiar() {
        nucleo.close()
        archivo.delete()
    }

    @Test
    fun `autenticar con credenciales validas guarda la sesion`() {
        val viewModel = LoginViewModel(nucleo)

        viewModel.cambiarCedula("999999999")
        viewModel.cambiarPassword(NucleoDePrueba.CLAVE_PRUEBA)
        viewModel.autenticar()

        assertNotNull(viewModel.sesion)
        assertEquals("Actor Test", viewModel.sesion?.nombre)
        assertNull(viewModel.error)
    }

    @Test
    fun `autenticar con contrasena incorrecta deja error y no guarda sesion`() {
        val viewModel = LoginViewModel(nucleo)

        viewModel.cambiarCedula("999999999")
        viewModel.cambiarPassword("no-es-la-clave")
        viewModel.autenticar()

        assertNull(viewModel.sesion)
        assertNotNull(viewModel.error)
    }

    @Test
    fun `cerrarSesion limpia cedula password y sesion`() {
        val viewModel = LoginViewModel(nucleo)
        viewModel.cambiarCedula("999999999")
        viewModel.cambiarPassword(NucleoDePrueba.CLAVE_PRUEBA)
        viewModel.autenticar()
        assertNotNull(viewModel.sesion)

        viewModel.cerrarSesion()

        assertNull(viewModel.sesion)
        assertEquals("", viewModel.cedula)
        assertEquals("", viewModel.password)
    }
}
