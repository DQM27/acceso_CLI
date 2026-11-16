package com.brisas.controlacceso

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.CarnePraind
import uniffi.control_acceso_mobile.EstadoCarnePraind
import uniffi.control_acceso_mobile.NucleoException

/// Guardar el carné, bajarlo sólo si hace falta y el aviso de "conectate a
/// internet" se prueban en Rust (`control_acceso::nube::praind`). Acá sólo
/// cómo el ViewModel mueve el estado.
@OptIn(ExperimentalCoroutinesApi::class)
class CarnePraindViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun preparar() = Dispatchers.setMain(dispatcher)

    @After
    fun limpiar() = Dispatchers.resetMain()

    private class FuenteFalsa(
        val conCarne: Set<Long> = setOf(1L),
        val error: String? = null,
    ) : FuenteCarnes {
        override fun estado(contratistaId: Long) =
            EstadoCarnePraind(tieneCarne = contratistaId in conCarne, guardado = false)

        override fun ver(contratistaId: Long): CarnePraind {
            error?.let { throw NucleoException.Rechazado(it) }
            return CarnePraind(tipoMime = "image/png", bytes = byteArrayOf(1, 2, 3))
        }
    }

    @Test
    fun ofreceVerCarneSoloSiElContratistaTiene() = runTest(dispatcher) {
        val vm = CarnePraindViewModel(FuenteFalsa(), dispatcher)
        vm.consultar(1L)
        advanceUntilIdle()
        assertTrue(vm.disponible.value)
        vm.consultar(2L)
        advanceUntilIdle()
        assertFalse(vm.disponible.value)
        vm.consultar(null)
        advanceUntilIdle()
        assertFalse(vm.disponible.value)
    }

    @Test
    fun abrirMuestraLaImagenYCerrarLaQuita() = runTest(dispatcher) {
        val vm = CarnePraindViewModel(FuenteFalsa(), dispatcher)
        vm.abrir(1L, "ANA PEREZ")
        assertEquals(VistaCarne.Cargando("ANA PEREZ"), vm.vista.value)
        advanceUntilIdle()
        val vista = vm.vista.value as VistaCarne.Imagen
        assertArrayEquals(byteArrayOf(1, 2, 3), vista.bytes)
        vm.cerrar()
        assertEquals(VistaCarne.Cerrada, vm.vista.value)
    }

    @Test
    fun sinInternetMuestraElAvisoDelNucleo() = runTest(dispatcher) {
        val aviso = "Para ver este carné por primera vez, conectate a internet."
        val vm = CarnePraindViewModel(FuenteFalsa(error = aviso), dispatcher)
        vm.abrir(1L, "ANA PEREZ")
        advanceUntilIdle()
        assertEquals(VistaCarne.Error("ANA PEREZ", aviso), vm.vista.value)
    }
}
