package com.brisas.controlacceso

import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.Nucleo

/// `LoginViewModel.autenticar` intenta una sincronización corta antes de
/// confirmar (ver su doc-comment) -- ya no es puramente síncrono, así que
/// estos tests necesitan el mismo patrón `runTest`/`StandardTestDispatcher`
/// que `HistorialViewModelTest`. Sin secreto de nube en el store de prueba,
/// la sincronización de fondo no toca red y el login sigue con lo que ya
/// validó local -- mismo comportamiento que en producción cuando el
/// dispositivo no tiene la nube configurada.
@OptIn(ExperimentalCoroutinesApi::class)
class LoginViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private lateinit var archivo: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        Dispatchers.setMain(dispatcher)
        archivo = File.createTempFile("login_test", ".db").apply { deleteOnExit() }
        nucleo = NucleoDePrueba.abrir(archivo, NucleoDePrueba.sqlUsuarioRoot())
    }

    @After
    fun limpiar() {
        nucleo.close()
        archivo.delete()
        Dispatchers.resetMain()
    }

    @Test
    fun `autenticar con credenciales validas guarda la sesion`() = runTest(dispatcher) {
        val viewModel = LoginViewModel(nucleo, SecretoDispositivoStoreDePrueba(), dispatcherIO = dispatcher)

        viewModel.cambiarCedula("999999999")
        viewModel.cambiarPassword(NucleoDePrueba.CLAVE_PRUEBA)
        viewModel.autenticar()
        advanceUntilIdle()

        assertNotNull(viewModel.sesion)
        assertEquals("Actor Test", viewModel.sesion?.nombre)
        assertNull(viewModel.error)
    }

    @Test
    fun `autenticar con contrasena incorrecta deja error y no guarda sesion`() = runTest(dispatcher) {
        val viewModel = LoginViewModel(nucleo, SecretoDispositivoStoreDePrueba(), dispatcherIO = dispatcher)

        viewModel.cambiarCedula("999999999")
        viewModel.cambiarPassword("no-es-la-clave")
        viewModel.autenticar()
        advanceUntilIdle()

        assertNull(viewModel.sesion)
        assertNotNull(viewModel.error)
    }

    @Test
    fun `cerrarSesion limpia cedula password y sesion`() = runTest(dispatcher) {
        val viewModel = LoginViewModel(nucleo, SecretoDispositivoStoreDePrueba(), dispatcherIO = dispatcher)
        viewModel.cambiarCedula("999999999")
        viewModel.cambiarPassword(NucleoDePrueba.CLAVE_PRUEBA)
        viewModel.autenticar()
        advanceUntilIdle()
        assertNotNull(viewModel.sesion)

        viewModel.cerrarSesion()

        assertNull(viewModel.sesion)
        assertEquals("", viewModel.cedula)
        assertEquals("", viewModel.password)
    }
}
