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
import uniffi.control_acceso_mobile.Nucleo

/// `ActivosViewModel` sí usa `viewModelScope.launch` + `withContext`, así
/// que estos tests corren con un `StandardTestDispatcher` compartido entre
/// `Dispatchers.Main` (lo que usa `viewModelScope`) y `dispatcherIO` (el
/// parámetro inyectado del ViewModel) — mismo scheduler para los dos, así
/// `advanceUntilIdle()` deja todo resuelto de forma determinística, sin
/// depender de hilos reales.
@OptIn(ExperimentalCoroutinesApi::class)
class ActivosViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private lateinit var archivo: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        Dispatchers.setMain(dispatcher)
        archivo = File.createTempFile("activos_test", ".db").apply { deleteOnExit() }
    }

    @After
    fun limpiar() {
        if (::nucleo.isInitialized) nucleo.close()
        archivo.delete()
        Dispatchers.resetMain()
    }

    @Test
    fun `base vacia no falla y no muestra a nadie adentro`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = ActivosViewModel(nucleo, dispatcherIO = dispatcher)

        advanceUntilIdle()

        assertTrue(viewModel.activos.isEmpty())
        assertNull(viewModel.error)
    }

    @Test
    fun `modo Entrada con texto busca en el catalogo completo de contratistas`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');",
            """
            INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
            ) VALUES ('111111111', 'Contratista Buscable', 1, 'SWAT', 0, 1);
            """.trimIndent(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        val viewModel = ActivosViewModel(nucleo, dispatcherIO = dispatcher)
        advanceUntilIdle()

        viewModel.cambiarTexto("Buscable")
        advanceUntilIdle()

        assertEquals(1, viewModel.resultadosBusqueda.size)
        assertEquals("Contratista Buscable", viewModel.resultadosBusqueda[0].nombre)
    }

    @Test
    fun `elegir un contratista sin praind pendiente prepara el formulario`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');",
            """
            INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
            ) VALUES ('111111111', 'Contratista Test', 1, 'SWAT', 0, 1);
            """.trimIndent(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        val viewModel = ActivosViewModel(nucleo, dispatcherIO = dispatcher)
        advanceUntilIdle()
        viewModel.cambiarTexto("Contratista")
        advanceUntilIdle()
        val contratista = viewModel.resultadosBusqueda.single()

        viewModel.elegir(contratista)
        advanceUntilIdle()

        val seleccion = viewModel.seleccionIngreso
        assertTrue(seleccion is SeleccionIngreso.Formulario)
        assertEquals("Contratista Test", (seleccion as SeleccionIngreso.Formulario).preparacion.nombre)
    }

    @Test
    fun `elegir un contratista sin acceso autorizado lo bloquea en vez de dejarlo continuar`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');",
            """
            INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
            ) VALUES ('222222222', 'Sin Acceso', 1, 'SWAT', 0, 0);
            """.trimIndent(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        val viewModel = ActivosViewModel(nucleo, dispatcherIO = dispatcher)
        advanceUntilIdle()
        viewModel.cambiarTexto("Sin Acceso")
        advanceUntilIdle()
        val contratista = viewModel.resultadosBusqueda.single()

        viewModel.elegir(contratista)
        advanceUntilIdle()

        assertTrue(viewModel.seleccionIngreso is SeleccionIngreso.Bloqueada)
    }

    @Test
    fun `cambiarModo limpia el texto y el mensaje anterior`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = ActivosViewModel(nucleo, dispatcherIO = dispatcher)
        advanceUntilIdle()
        viewModel.cambiarTexto("algo")
        advanceUntilIdle()

        viewModel.cambiarModo(ModoBusqueda.SALIDA_GAFETE)
        advanceUntilIdle()

        assertEquals(ModoBusqueda.SALIDA_GAFETE, viewModel.modo)
        assertEquals("", viewModel.texto)
    }
}
