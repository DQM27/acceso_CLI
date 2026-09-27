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
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.Nucleo

/// Punto M7 de la auditoría móvil: este ViewModel no tenía tests. Nada de
/// acá toca la red: la entrega con secreto válido hace un chequeo en vivo
/// contra la nube, por eso sólo se prueba el camino sin secreto (que falla
/// antes de salir del teléfono).
@OptIn(ExperimentalCoroutinesApi::class)
class GafetesProvisionalesViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private lateinit var archivo: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        Dispatchers.setMain(dispatcher)
        archivo = File.createTempFile("gafetes_provisionales_test", ".db").apply { deleteOnExit() }
    }

    @After
    fun limpiar() {
        if (::nucleo.isInitialized) nucleo.close()
        archivo.delete()
        Dispatchers.resetMain()
    }

    private fun abrir() {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            """
            INSERT INTO encargados_ruta (codigo_empleado, nombre, activo, uuid)
            VALUES ('5040017', 'Michael Araya Retana', 1, 'encargado-araya');
            """.trimIndent(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
    }

    private fun viewModel(secreto: String? = null) =
        GafetesProvisionalesViewModel(nucleo, SecretoDispositivoStoreDePrueba(secreto), dispatcherIO = dispatcher)

    @Test
    fun `base vacia no falla`() = runTest(dispatcher) {
        abrir()
        val vm = viewModel()
        advanceUntilIdle()
        assertTrue(vm.activos.isEmpty())
        assertNull(vm.error)
        assertFalse(vm.cargando)
    }

    @Test
    fun `busca y elige encargado`() = runTest(dispatcher) {
        abrir()
        val vm = viewModel()
        advanceUntilIdle()
        vm.cambiarTextoEncargado("araya")
        advanceUntilIdle()
        val encargado = vm.resultadosEncargado.single()
        vm.elegirEncargado(encargado)
        assertEquals("Michael Araya Retana · 5040017", vm.textoEncargado)
        assertEquals(encargado, vm.encargadoSeleccionado)
        assertTrue(vm.resultadosEncargado.isEmpty())
    }

    @Test
    fun `entregar sin encargado elegido no hace nada`() = runTest(dispatcher) {
        abrir()
        val vm = viewModel()
        advanceUntilIdle()
        var exito = false
        vm.entregar(5) { exito = true }
        advanceUntilIdle()
        assertFalse(exito)
        assertFalse(vm.registrando)
    }

    @Test
    fun `entregar sin secreto del dispositivo muestra el error y no presta`() = runTest(dispatcher) {
        abrir()
        val vm = viewModel(secreto = null)
        advanceUntilIdle()
        vm.cambiarTextoEncargado("araya")
        advanceUntilIdle()
        vm.elegirEncargado(vm.resultadosEncargado.single())
        var exito = false
        vm.entregar(5) { exito = true }
        advanceUntilIdle()
        assertFalse(exito)
        assertEquals(SecretoDispositivoNoEncontradoException().message, vm.error)
        assertTrue(nucleo.listarGafetesProvisionalesActivos().isEmpty())
    }

    @Test
    fun `devolucion local cierra el prestamo`() = runTest(dispatcher) {
        abrir()
        nucleo.entregarGafeteProvisional(1, 5)
        val vm = viewModel()
        advanceUntilIdle()
        val fila = vm.activos.single()
        assertTrue(fila is FilaGafeteProvisionalActiva.Local)

        vm.registrarDevolucion(fila)
        advanceUntilIdle()

        assertTrue(vm.activos.isEmpty())
        assertNull(vm.error)
    }
}
