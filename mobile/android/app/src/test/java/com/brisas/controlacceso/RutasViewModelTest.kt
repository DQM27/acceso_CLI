package com.brisas.controlacceso

import java.io.File
import java.time.LocalDate
import java.time.ZoneId
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
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.DecisionRetornoViaje
import uniffi.control_acceso_mobile.Nucleo

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

    private fun sqlVehiculoC12345(): String =
        """
        INSERT INTO vehiculos_ruta (numero_unidad, placa, activo, uuid)
        VALUES ('U-10', 'C12345', 1, 'vehiculo-c12345');
        """.trimIndent()

    /// Mismo formato que `FechaDocumento.aTextoDDMMYYYYRuta` en
    /// `PantallaRutas.kt` -- "hoy" en la zona de Costa Rica, no la del
    /// sistema del runner (UTC en CI). Ver el comentario largo que tenía
    /// este mismo cálculo antes de moverse acá.
    private fun hoyTextoRuta(): String {
        val hoy = LocalDate.now(ZoneId.of("America/Costa_Rica"))
        return "%02d-%02d-%04d".format(hoy.dayOfMonth, hoy.monthValue, hoy.year)
    }

    /// Completa el paso "Encargado" del checklist buscando y eligiendo el
    /// fixture de Araya -- repetido en varios tests, mismo motivo que
    /// `nucleo_con_actor_y_ruta_79` en el núcleo.
    private suspend fun TestScope.elegirEncargadoAraya(viewModel: RutasViewModel) {
        viewModel.cambiarTextoEncargado("araya")
        advanceUntilIdle()
        viewModel.elegirEncargado(viewModel.resultadosEncargado.single())
    }

    private suspend fun TestScope.elegirVehiculoC12345(viewModel: RutasViewModel) {
        viewModel.cambiarTextoVehiculo("C12345")
        advanceUntilIdle()
        viewModel.elegirVehiculo(viewModel.resultadosVehiculo.single())
        advanceUntilIdle()
    }

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

    // ---- Documento(s) de ruta ----

    @Test
    fun `un documento nuevo arranca sin fecha -- nunca defaultea a hoy`() = runTest(dispatcher) {
        // Regresión del hallazgo de esta ronda: el bloqueo por fecha
        // vencida no se activaba para un documento cuya fecha no se pudo
        // leer del OCR, porque el campo defaulteaba a "hoy" en silencio.
        nucleo = NucleoDePrueba.abrir(archivo, NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = viewModel()
        advanceUntilIdle()

        assertEquals(1, viewModel.documentos.size)
        assertEquals("", viewModel.documentos.single().fechaTexto)
    }

    @Test
    fun `agregarDocumento suma un documento y quitarDocumento nunca deja la lista vacia`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = viewModel()
        advanceUntilIdle()

        viewModel.agregarDocumento()
        assertEquals(2, viewModel.documentos.size)

        val idPrimero = viewModel.documentos[0].id
        viewModel.quitarDocumento(idPrimero)
        assertEquals(1, viewModel.documentos.size)

        val idUnico = viewModel.documentos.single().id
        viewModel.quitarDocumento(idUnico)
        assertEquals("nunca queda sin ningún documento", 1, viewModel.documentos.size)
    }

    @Test
    fun `cambiarTextoRutaDocumento encuentra coincidencia parcial del numero`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, sqlRuta79(), NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = viewModel()
        advanceUntilIdle()
        val id = viewModel.documentos.single().id

        viewModel.cambiarTextoRutaDocumento(id, "7")
        advanceUntilIdle()

        val documento = viewModel.documentos.single()
        assertEquals(1, documento.resultadosRuta.size)
        assertEquals(79L, documento.resultadosRuta[0].numero)
    }

    @Test
    fun `usarNumeroRutaEscaneadoDocumento con coincidencia exacta unica la elige sola`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, sqlRuta79(), NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = viewModel()
        advanceUntilIdle()
        val id = viewModel.documentos.single().id

        viewModel.usarNumeroRutaEscaneadoDocumento(id, 79)
        advanceUntilIdle()

        val documento = viewModel.documentos.single()
        assertEquals(79L, documento.rutaSeleccionada?.numero)
        assertEquals("79", documento.textoRuta)
    }

    @Test
    fun `registrarSalida y registrarRetorno redondean el viaje`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            sqlRuta79(),
            sqlEncargadoAraya(),
            sqlVehiculoC12345(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
        val viewModel = viewModel()
        advanceUntilIdle()

        elegirEncargadoAraya(viewModel)
        val id = viewModel.documentos.single().id
        viewModel.cambiarTextoRutaDocumento(id, "79")
        advanceUntilIdle()
        viewModel.elegirRutaDocumento(id, viewModel.documentos.single().resultadosRuta.single())
        viewModel.cambiarNumeroDocumento(id, "700101452")
        // Debe ser "hoy" para no chocar con el bloqueo transitorio por
        // documento vencido (`RutaServiceError::DocumentoRequiereAutorizacion`).
        viewModel.cambiarFechaDocumento(id, hoyTextoRuta())
        elegirVehiculoC12345(viewModel)

        var onExitoLlamado = false
        viewModel.registrarSalida(onExito = { onExitoLlamado = true })
        advanceUntilIdle()

        assertTrue(onExitoLlamado)
        assertNull(viewModel.error)
        assertEquals(1, viewModel.activas.size)
        val salida = viewModel.activas.single()
        assertEquals("C12345", salida.vehiculoPlaca)
        assertEquals(1, viewModel.tramosPorViaje[salida.viajeId]?.size)

        viewModel.registrarRetorno(salida, DecisionRetornoViaje.NO_VUELVE_A_SALIR)
        advanceUntilIdle()

        assertTrue(viewModel.activas.isEmpty())
    }

    @Test
    fun `tramosPorViaje trae el historial completo tras una recarga`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            sqlRuta79(),
            sqlEncargadoAraya(),
            sqlVehiculoC12345(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
        val viewModel = viewModel()
        advanceUntilIdle()

        elegirEncargadoAraya(viewModel)
        val id = viewModel.documentos.single().id
        viewModel.cambiarTextoRutaDocumento(id, "79")
        advanceUntilIdle()
        viewModel.elegirRutaDocumento(id, viewModel.documentos.single().resultadosRuta.single())
        viewModel.cambiarNumeroDocumento(id, "700101452")
        viewModel.cambiarFechaDocumento(id, hoyTextoRuta())
        elegirVehiculoC12345(viewModel)
        viewModel.registrarSalida(onExito = {})
        advanceUntilIdle()
        val primerTramo = viewModel.activas.single()

        // "Sí, misma ruta" -- el viaje sigue abierto, listo para un tramo nuevo.
        viewModel.registrarRetorno(primerTramo, DecisionRetornoViaje.MISMA_RUTA)
        advanceUntilIdle()

        elegirEncargadoAraya(viewModel)
        val idSegundoDocumento = viewModel.documentos.single().id
        viewModel.cambiarTextoRutaDocumento(idSegundoDocumento, "79")
        advanceUntilIdle()
        viewModel.elegirRutaDocumento(idSegundoDocumento, viewModel.documentos.single().resultadosRuta.single())
        viewModel.cambiarNumeroDocumento(idSegundoDocumento, "700101452")
        viewModel.cambiarFechaDocumento(idSegundoDocumento, hoyTextoRuta())
        elegirVehiculoC12345(viewModel)
        assertTrue("debe detectar el viaje abierto para continuarlo", viewModel.viajeAbiertoParaVehiculo != null)
        viewModel.alternarContinuarViaje(true)
        viewModel.registrarSalida(onExito = {})
        advanceUntilIdle()

        val segundoTramo = viewModel.activas.single()
        assertEquals(primerTramo.viajeId, segundoTramo.viajeId)
        val historial = viewModel.tramosPorViaje[segundoTramo.viajeId]
        assertEquals(2, historial?.size)
        assertEquals(primerTramo.id, historial?.get(0)?.id)
        assertNotNull(historial?.get(0)?.fechaHoraRetorno)
        assertEquals(segundoTramo.id, historial?.get(1)?.id)
        assertNull(historial?.get(1)?.fechaHoraRetorno)
    }

    @Test
    fun `registrarSalida con documento vencido y sin correo falla y deja un error legible`() = runTest(dispatcher) {
        // El botón "Confirmar salida" de la pantalla real ya bloquea este
        // caso (`bloqueadoPorFecha`), pero `registrarSalida` no vuelve a
        // validarlo -- confía en el resguardo del núcleo, mismo criterio
        // que el resto de los servicios (no confiar sólo en la UI).
        nucleo = NucleoDePrueba.abrir(
            archivo,
            sqlRuta79(),
            sqlEncargadoAraya(),
            sqlVehiculoC12345(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
        val viewModel = viewModel()
        advanceUntilIdle()

        elegirEncargadoAraya(viewModel)
        val id = viewModel.documentos.single().id
        viewModel.cambiarTextoRutaDocumento(id, "79")
        advanceUntilIdle()
        viewModel.elegirRutaDocumento(id, viewModel.documentos.single().resultadosRuta.single())
        viewModel.cambiarNumeroDocumento(id, "700101452")
        viewModel.cambiarFechaDocumento(id, "01-01-2020")
        elegirVehiculoC12345(viewModel)

        var onExitoLlamado = false
        viewModel.registrarSalida(onExito = { onExitoLlamado = true })
        advanceUntilIdle()

        assertTrue(viewModel.activas.isEmpty())
        assertNotNull(viewModel.error)
        assertTrue(!onExitoLlamado)
    }

    // ---- Vehículo / viaje abierto ----

    @Test
    fun `elegirVehiculo detecta un viaje abierto para la misma placa`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            sqlRuta79(),
            sqlEncargadoAraya(),
            sqlVehiculoC12345(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
        val viewModel = viewModel()
        advanceUntilIdle()

        // Sin viaje abierto todavía -- primera salida del día.
        elegirVehiculoC12345(viewModel)
        assertNull(viewModel.viajeAbiertoParaVehiculo)

        elegirEncargadoAraya(viewModel)
        val id = viewModel.documentos.single().id
        viewModel.cambiarTextoRutaDocumento(id, "79")
        advanceUntilIdle()
        viewModel.elegirRutaDocumento(id, viewModel.documentos.single().resultadosRuta.single())
        viewModel.cambiarNumeroDocumento(id, "700101452")
        viewModel.cambiarFechaDocumento(id, hoyTextoRuta())
        viewModel.registrarSalida(onExito = {})
        advanceUntilIdle()

        // El formulario se limpia solo tras registrar -- el guardia vuelve
        // a elegir la misma placa (ej. la unidad volvió y dijo "misma
        // ruta" al confirmar el retorno).
        elegirVehiculoC12345(viewModel)

        val viajeAbierto = viewModel.viajeAbiertoParaVehiculo
        assertNotNull(viajeAbierto)
        assertEquals("Michael Araya Retana", viajeAbierto?.encargadoNombre)
    }
}
