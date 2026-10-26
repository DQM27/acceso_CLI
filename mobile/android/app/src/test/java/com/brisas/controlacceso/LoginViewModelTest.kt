package com.brisas.controlacceso

import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.Nucleo

/// `LoginViewModel.autenticar` intenta una sincronización corta antes de
/// confirmar (ver su doc-comment) -- ya no es puramente síncrono, así que
/// estos tests necesitan el mismo patrón `runTest`/`StandardTestDispatcher`
/// que el resto de los ViewModelTest de la app. El teléfono de prueba no está vinculado:
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
        val viewModel = LoginViewModel(nucleo, dispatcherIO = dispatcher)

        viewModel.cambiarCedula("999999999")
        viewModel.cambiarPassword(NucleoDePrueba.CLAVE_PRUEBA)
        viewModel.autenticar()
        advanceUntilIdle()

        assertNotNull(viewModel.sesion)
        assertNotNull(viewModel.propietarioSesion)
        assertEquals("Actor Test", viewModel.sesion?.nombre)
        assertEquals("", viewModel.password)
        assertNull(viewModel.error)
    }

    @Test
    fun `autenticar con contrasena incorrecta marca el campo y no guarda sesion`() = runTest(dispatcher) {
        val viewModel = LoginViewModel(nucleo, dispatcherIO = dispatcher)

        viewModel.cambiarCedula("999999999")
        viewModel.cambiarPassword("no-es-la-clave")
        viewModel.autenticar()
        advanceUntilIdle()

        assertNull(viewModel.sesion)
        // Va debajo del campo, no en el aviso general.
        assertNotNull(viewModel.errorCredenciales)
        assertNull(viewModel.error)
        assertEquals(1, viewModel.intentosFallidos)
    }

    @Test
    fun `volver a escribir la contrasena borra el error del campo`() = runTest(dispatcher) {
        val viewModel = LoginViewModel(nucleo, dispatcherIO = dispatcher)
        viewModel.cambiarCedula("999999999")
        viewModel.cambiarPassword("no-es-la-clave")
        viewModel.autenticar()
        advanceUntilIdle()
        assertNotNull(viewModel.errorCredenciales)

        viewModel.cambiarPassword("otra")

        assertNull(viewModel.errorCredenciales)
    }

    @Test
    fun `contrasena temporal pasa al paso de elegir una nueva sin abrir sesion`() = runTest(dispatcher) {
        val viewModel = viewModelConTemporal()

        assertTrue(viewModel.cambiandoTemporal)
        assertNull(viewModel.sesion)
        assertNull(viewModel.error)
        assertNull(viewModel.errorCredenciales)
        // La temporal no queda en el campo visible.
        assertEquals("", viewModel.password)
        assertFalse(viewModel.revision.valida)
    }

    @Test
    fun `los requisitos de la nueva se actualizan mientras se escribe`() = runTest(dispatcher) {
        val viewModel = viewModelConTemporal()

        viewModel.cambiarPasswordNueva(NucleoDePrueba.CLAVE_PRUEBA)
        viewModel.cambiarConfirmacion(NucleoDePrueba.CLAVE_PRUEBA)
        assertTrue(viewModel.revision.largoSuficiente)
        assertTrue(viewModel.revision.coinciden)
        assertFalse(viewModel.revision.distintaDeLaTemporal)
        assertFalse(viewModel.revision.valida)

        viewModel.cambiarPasswordNueva("una-nueva-segura")
        viewModel.cambiarConfirmacion("una-nueva-segura")
        assertTrue(viewModel.revision.valida)
    }

    @Test
    fun `guardar con una nueva que no cumple no intenta el cambio`() = runTest(dispatcher) {
        val viewModel = viewModelConTemporal()
        viewModel.cambiarPasswordNueva("corta")
        viewModel.cambiarConfirmacion("corta")

        viewModel.guardarPasswordNueva()
        advanceUntilIdle()

        assertFalse(viewModel.guardandoNueva)
        assertTrue(viewModel.cambiandoTemporal)
        assertNull(viewModel.errorCambio)
        assertNull(viewModel.sesion)
    }

    @Test
    fun `volver desde la temporal deja el login con la cedula puesta`() = runTest(dispatcher) {
        val viewModel = viewModelConTemporal()
        viewModel.cambiarPasswordNueva("una-nueva-segura")

        viewModel.cancelarCambioTemporal()

        assertFalse(viewModel.cambiandoTemporal)
        assertEquals("", viewModel.passwordNueva)
        assertEquals(CEDULA_TEMPORAL, viewModel.cedula)
    }

    /// Cuenta con la temporal cacheada (`password_temporal_cacheada = 1`):
    /// el núcleo la reconoce sin red, igual que en el test de Rust.
    private fun TestScope.viewModelConTemporal(): LoginViewModel {
        nucleo.close()
        archivo.delete()
        archivo = File.createTempFile("login_temporal_test", ".db").apply { deleteOnExit() }
        nucleo = NucleoDePrueba.abrir(
            archivo,
            """
            INSERT INTO usuarios (
                cedula, nombre, password_hash, rol, activo,
                password_hash_confirmado_en, password_temporal_cacheada
            ) VALUES (
                '$CEDULA_TEMPORAL', 'Usuario Nuevo', '${NucleoDePrueba.HASH_CLAVE_PRUEBA}', 'OPERADOR', 1,
                strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), 1
            );
            """.trimIndent(),
        )
        val viewModel = LoginViewModel(nucleo, dispatcherIO = dispatcher)
        viewModel.cambiarCedula(CEDULA_TEMPORAL)
        viewModel.cambiarPassword(NucleoDePrueba.CLAVE_PRUEBA)
        viewModel.autenticar()
        advanceUntilIdle()
        return viewModel
    }

    private companion object {
        const val CEDULA_TEMPORAL = "777777777"
    }

    @Test
    fun `cerrarSesion limpia cedula password y sesion`() = runTest(dispatcher) {
        val viewModel = LoginViewModel(nucleo, dispatcherIO = dispatcher)
        viewModel.cambiarCedula("999999999")
        viewModel.cambiarPassword(NucleoDePrueba.CLAVE_PRUEBA)
        viewModel.autenticar()
        advanceUntilIdle()
        assertNotNull(viewModel.sesion)

        viewModel.cerrarSesion()

        assertNull(viewModel.sesion)
        assertNull(viewModel.propietarioSesion)
        assertEquals("", viewModel.cedula)
        assertEquals("", viewModel.password)
    }

    @Test
    fun `el cierre por inactividad deja la cedula y borra la contrasena`() = runTest(dispatcher) {
        val viewModel = LoginViewModel(nucleo, dispatcherIO = dispatcher)
        viewModel.cambiarCedula("999999999")
        viewModel.cambiarPassword(NucleoDePrueba.CLAVE_PRUEBA)
        viewModel.autenticar()
        advanceUntilIdle()

        viewModel.cerrarSesion(conservarCedula = true)

        assertNull(viewModel.sesion)
        assertEquals("999999999", viewModel.cedula)
        assertEquals("", viewModel.password)
    }

    @Test
    fun `doble toque durante autenticacion no abre dos intentos`() = runTest(dispatcher) {
        val viewModel = LoginViewModel(nucleo, dispatcherIO = dispatcher)
        viewModel.cambiarCedula("999999999")
        viewModel.cambiarPassword(NucleoDePrueba.CLAVE_PRUEBA)

        viewModel.autenticar()
        viewModel.autenticar()
        advanceUntilIdle()

        assertNotNull(viewModel.sesion)
        assertNull(viewModel.error)
    }
}
