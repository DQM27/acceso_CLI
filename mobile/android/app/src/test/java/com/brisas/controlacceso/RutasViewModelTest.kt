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
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.SolicitudSalidaRuta

/// Mismo criterio que `ActivosViewModelTest` -- un `StandardTestDispatcher`
/// compartido entre `Dispatchers.Main` y `dispatcherIO` para que
/// `advanceUntilIdle()` deje todo resuelto sin depender de hilos reales.
@OptIn(ExperimentalCoroutinesApi::class)
class RutasViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private lateinit var archivo: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        Dispatchers.setMain(dispatcher)
        archivo = File.createTempFile("rutas_test", ".db").apply { deleteOnExit() }
    }

    @After
    fun limpiar() {
        if (::nucleo.isInitialized) nucleo.close()
        archivo.delete()
        Dispatchers.resetMain()
    }

    private fun viewModel(): RutasViewModel = RutasViewModel(nucleo, dispatcherIO = dispatcher)

    private fun sqlRuta79(): String =
        "INSERT INTO rutas (numero, activo, uuid) VALUES (79, 1, 'ruta-79');"

    private fun sqlEncargadoAraya(): String =
        """
        INSERT INTO encargados_ruta (codigo_empleado, nombre, activo, uuid)
        VALUES ('5040017', 'Michael Araya Retana', 1, 'encargado-araya');
        """.trimIndent()

    @Test
    fun `base vacia no falla y no muestra rutas activas`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = viewModel()

        advanceUntilIdle()

        assertTrue(viewModel.activas.isEmpty())
        assertNull(viewModel.error)
    }

    @Test
    fun `cambiarTextoEncargado encuentra por nombre parcial`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, sqlEncargadoAraya(), NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = viewModel()
        advanceUntilIdle()

        viewModel.cambiarTextoEncargado("araya")
        advanceUntilIdle()

        assertEquals(1, viewModel.resultadosEncargado.size)
        assertEquals("Michael Araya Retana", viewModel.resultadosEncargado[0].nombre)
    }

    @Test
    fun `elegirEncargado fija el texto y la seleccion`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, sqlEncargadoAraya(), NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = viewModel()
        advanceUntilIdle()
        viewModel.cambiarTextoEncargado("araya")
        advanceUntilIdle()
        val encargado = viewModel.resultadosEncargado.single()

        viewModel.elegirEncargado(encargado)

        assertEquals("Michael Araya Retana", viewModel.textoEncargado)
        assertEquals(encargado, viewModel.encargadoSeleccionado)
        assertTrue(viewModel.resultadosEncargado.isEmpty())
    }

    @Test
    fun `cambiarTextoRuta encuentra coincidencia parcial del numero`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, sqlRuta79(), NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = viewModel()
        advanceUntilIdle()

        viewModel.cambiarTextoRuta("7")
        advanceUntilIdle()

        assertEquals(1, viewModel.resultadosRuta.size)
        assertEquals(79L, viewModel.resultadosRuta[0].numero)
    }

    @Test
    fun `usarNumeroRutaEscaneado con coincidencia exacta unica la elige sola`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, sqlRuta79(), NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = viewModel()
        advanceUntilIdle()

        viewModel.usarNumeroRutaEscaneado(79)
        advanceUntilIdle()

        assertEquals(79L, viewModel.rutaSeleccionada?.numero)
        assertEquals("79", viewModel.textoRuta)
    }

    @Test
    fun `registrarSalida y registrarRetorno redondean el viaje`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            sqlRuta79(),
            sqlEncargadoAraya(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
        val viewModel = viewModel()
        advanceUntilIdle()

        var onExitoLlamado = false
        viewModel.registrarSalida(
            SolicitudSalidaRuta(
                vehiculoPlaca = "C12345",
                vehiculoNumeroUnidad = null,
                encargadoNombre = "Michael Araya Retana",
                encargadoCodigoEmpleado = "5040017",
                numeroRuta = 79L,
                subNumero = 1L,
                numeroDocumento = "700101452",
                // Debe ser "hoy" para no chocar con el bloqueo transitorio por
                // documento vencido (`RutaServiceError::DocumentoRequiereAutorizacion`)
                // -- una fecha fija se vence sola al día siguiente de escribirla.
                // Zona horaria explícita de Costa Rica, no la del sistema del
                // runner (UTC en CI) -- el núcleo Rust define "hoy" con esa
                // zona (`fecha_costa_rica`), y entre 00:00 y 06:00 UTC ese
                // "hoy" todavía es "ayer" en CR, lo que hacía fallar este test
                // de forma intermitente según la hora en que corriera CI.
                fechaDocumento = java.time.LocalDate.now(java.time.ZoneId.of("America/Costa_Rica")).toString(),
                tieneCorreoAutorizacion = false,
            ),
            onExito = { onExitoLlamado = true },
        )
        advanceUntilIdle()

        assertTrue(onExitoLlamado)
        assertNull(viewModel.error)
        assertEquals(1, viewModel.activas.size)
        val salida = viewModel.activas.single()
        assertEquals(79L, salida.numeroRuta)

        viewModel.registrarRetorno(salida)
        advanceUntilIdle()

        assertTrue(viewModel.activas.isEmpty())
    }

    @Test
    fun `registrarSalida con numero de ruta inexistente falla y deja un error legible`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, NucleoDePrueba.sqlUsuarioRoot())
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
        val viewModel = viewModel()
        advanceUntilIdle()

        var onExitoLlamado = false
        viewModel.registrarSalida(
            SolicitudSalidaRuta(
                vehiculoPlaca = "C12345",
                vehiculoNumeroUnidad = null,
                encargadoNombre = "Sin Catalogo",
                encargadoCodigoEmpleado = null,
                numeroRuta = 222L,
                subNumero = 1L,
                numeroDocumento = "700101452",
                fechaDocumento = "2026-09-15",
                tieneCorreoAutorizacion = false,
            ),
            onExito = { onExitoLlamado = true },
        )
        advanceUntilIdle()

        assertTrue(viewModel.activas.isEmpty())
        assertNotNull(viewModel.error)
        assertTrue(onExitoLlamado == false)
    }
}
