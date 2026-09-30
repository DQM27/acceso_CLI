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
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.Nucleo

/// Punto M7 de la auditoría móvil. Sólo el camino que no sale del
/// teléfono: `vincular` con un código completo habla con Supabase
/// (`device-vincular`), y un test unitario nunca debe tocar la nube.
@OptIn(ExperimentalCoroutinesApi::class)
class PrimerArranqueViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private lateinit var archivo: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        Dispatchers.setMain(dispatcher)
        archivo = File.createTempFile("primer_arranque_test", ".db").apply { deleteOnExit() }
        nucleo = NucleoDePrueba.abrir(archivo)
    }

    @After
    fun limpiar() {
        nucleo.close()
        archivo.delete()
        Dispatchers.resetMain()
    }

    @Test
    fun `un codigo incompleto no intenta vincular`() = runTest(dispatcher) {
        val vm = PrimerArranqueViewModel(
            nucleo,
            MetadatosDispositivoLocal("hw", "Teléfono", "Android", "1", "1.0.0"),
            dispatcherIO = dispatcher,
        )
        var listo = false

        vm.vincular("K7QM-R4") { listo = true }
        advanceUntilIdle()

        assertFalse(listo)
        assertFalse(vm.vinculando)
        assertNull(vm.error)
    }

    @Test
    fun `sin clave vinculada la nube no esta configurada`() {
        assertFalse(nucleo.nubeConfigurada())
        assertNull(nucleo.dispositivoVinculado())
    }
}
