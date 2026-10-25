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
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.Nucleo

/// Lo que decide la reconexión se prueba en el núcleo
/// (`application::reconexion`). Acá, sólo caminos que no salen del
/// teléfono: un test unitario nunca debe tocar la nube.
@OptIn(ExperimentalCoroutinesApi::class)
class ReconexionViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private lateinit var archivo: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        Dispatchers.setMain(dispatcher)
        archivo = File.createTempFile("reconexion_test", ".db").apply { deleteOnExit() }
        nucleo = NucleoDePrueba.abrir(archivo)
    }

    @After
    fun limpiar() {
        nucleo.close()
        archivo.delete()
        Dispatchers.resetMain()
    }

    private fun viewModel() = ReconexionViewModel(
        nucleo,
        MetadatosDispositivoLocal("hw", "Teléfono", "Android", "1", "1.0.0"),
        dispatcherIO = dispatcher,
    )

    @Test
    fun `un telefono que no fue retirado no ofrece reconectar`() {
        assertFalse(nucleo.equipoRetirado())
    }

    @Test
    fun `un codigo incompleto no intenta reconectar`() = runTest(dispatcher) {
        val vm = viewModel()
        var reconectado = false

        vm.reconectar("K7QM-R4") { reconectado = true }
        advanceUntilIdle()

        assertFalse(reconectado)
        assertFalse(vm.enviando)
        assertNull(vm.error)
    }

    @Test
    fun `sin estar retirado el nucleo se niega y la pantalla muestra el motivo`() = runTest(dispatcher) {
        val vm = viewModel()
        var reconectado = false

        vm.reconectar("K7QM-R4XT-2P") { reconectado = true }
        advanceUntilIdle()

        assertFalse(reconectado)
        assertEquals(PasoReconexion.Codigo, vm.paso)
        assertEquals("Este equipo no está retirado: no hace falta reconectarlo", vm.error)
    }

    @Test
    fun `el aviso dice registro o registros`() {
        assertEquals("1 registro", textoRegistros(1uL))
        assertEquals("3 registros", textoRegistros(3uL))
    }
}
