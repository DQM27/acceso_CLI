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
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.MedioIngreso
import uniffi.control_acceso_mobile.Nucleo

@OptIn(ExperimentalCoroutinesApi::class)
class HistorialViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private lateinit var archivo: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        Dispatchers.setMain(dispatcher)
        archivo = File.createTempFile("historial_test", ".db").apply { deleteOnExit() }
    }

    @After
    fun limpiar() {
        if (::nucleo.isInitialized) nucleo.close()
        archivo.delete()
        Dispatchers.resetMain()
    }

    @Test
    fun `base vacia no falla y no muestra movimientos`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = HistorialViewModel(nucleo, dispatcherIO = dispatcher)

        advanceUntilIdle()

        assertTrue(viewModel.movimientos.isEmpty())
        assertNull(viewModel.error)
    }

    @Test
    fun `encuentra un movimiento reciente sin salida registrada`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');",
            """
            INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
            ) VALUES ('111111111', 'Contratista Historial', 1, 'SWAT', 0, 1);
            """.trimIndent(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA)
        nucleo.registrarIngreso(1, MedioIngreso.CAMINANDO, null)

        val viewModel = HistorialViewModel(nucleo, dispatcherIO = dispatcher)
        advanceUntilIdle()

        assertEquals(1, viewModel.movimientos.size)
        assertEquals("Contratista Historial", viewModel.movimientos[0].contratistaNombre)
        assertNull(viewModel.movimientos[0].fechaHoraSalida)
    }

    @Test
    fun `cambiarTexto sin coincidencias deja la lista vacia`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');",
            """
            INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
            ) VALUES ('111111111', 'Contratista Historial', 1, 'SWAT', 0, 1);
            """.trimIndent(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA)
        nucleo.registrarIngreso(1, MedioIngreso.CAMINANDO, null)

        val viewModel = HistorialViewModel(nucleo, dispatcherIO = dispatcher)
        advanceUntilIdle()

        viewModel.cambiarTexto("no existe nadie con este nombre")
        advanceUntilIdle()

        assertTrue(viewModel.movimientos.isEmpty())
    }
}
