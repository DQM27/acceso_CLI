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
import uniffi.control_acceso_mobile.TipoIngreso

/// Mismo criterio que `RutasViewModelTest`: un `StandardTestDispatcher`
/// compartido entre `Dispatchers.Main` y `dispatcherIO`.
@OptIn(ExperimentalCoroutinesApi::class)
class NuevoContratistaViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private lateinit var archivo: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        Dispatchers.setMain(dispatcher)
        archivo = File.createTempFile("nuevo_contratista_test", ".db").apply { deleteOnExit() }
    }

    @After
    fun limpiar() {
        if (::nucleo.isInitialized) nucleo.close()
        archivo.delete()
        Dispatchers.resetMain()
    }

    private fun abrirConEmpresas() {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            "INSERT INTO empresas (nombre) VALUES ('Sodexo');",
            "INSERT INTO empresas (nombre) VALUES ('Expenic Ing S.A');",
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
    }

    private fun viewModel() = NuevoContratistaViewModel(nucleo, dispatcherIO = dispatcher)

    private fun praind(empresa: String?, vence: FechaDocumento?) = DocumentoDetectado(
        tipo = TipoDocumento.CARNET_INDUCCION_PRAIND,
        numeroDocumento = "701000000",
        nombre = "Ana Rojas Vega",
        empresa = empresa,
        vencimiento = vence,
    )

    @Test
    fun `carga las empresas al abrir`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        advanceUntilIdle()
        assertEquals(listOf("Expenic Ing S.A", "Sodexo"), vm.empresas.map { it.nombre }.sorted())
        assertNull(vm.error)
    }

    @Test
    fun `el PRAIND escaneado rellena todo y elige la empresa`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        advanceUntilIdle()

        vm.aplicarDocumentoEscaneado(praind("sodexo", FechaDocumento(3, 8, 2099)))

        assertEquals("701000000", vm.cedula)
        assertEquals("ANA ROJAS VEGA", vm.nombre)
        assertEquals("Sodexo", vm.empresaSeleccionada?.nombre)
        assertEquals("03-08-2099", vm.fechaPraind)
        assertNull(vm.empresaSugeridaTexto)
        assertFalse(vm.praindVencido)
    }

    @Test
    fun `empresa del carnet que no existe queda como sugerencia`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        advanceUntilIdle()
        vm.aplicarDocumentoEscaneado(praind("Constructora Nueva", null))
        assertNull(vm.empresaSeleccionada)
        assertEquals("Constructora Nueva", vm.empresaSugeridaTexto)
    }

    @Test
    fun `otro documento que no sea PRAIND se rechaza sin tocar campos`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        advanceUntilIdle()
        vm.cambiarNombre("previo")
        vm.aplicarDocumentoEscaneado(DocumentoDetectado(TipoDocumento.CEDULA_NACIONAL, "123456789", nombre = "X"))
        assertEquals("Documento inválido", vm.error)
        assertEquals("PREVIO", vm.nombre)
    }

    @Test
    fun `PRAIND vencido bloquea el guardado`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        advanceUntilIdle()
        vm.aplicarDocumentoEscaneado(praind("Sodexo", FechaDocumento(1, 1, 2000)))
        assertTrue(vm.praindVencido)
        var guardado = false
        vm.guardar { guardado = true }
        advanceUntilIdle()
        assertFalse(guardado)
        assertEquals("El PRAIND está vencido — ingrese una fecha vigente", vm.error)
    }

    @Test
    fun `sin empresa pide elegirla`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        advanceUntilIdle()
        vm.cambiarCedula("7-0100-0000")
        vm.cambiarNombre("ana")
        vm.guardar { }
        assertEquals("Elija la empresa", vm.error)
        assertEquals("701000000", vm.cedula)
    }

    @Test
    fun `sin cedula el mensaje lo da el nucleo`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        advanceUntilIdle()
        vm.elegirEmpresa(vm.empresas.first())
        vm.cambiarTipoIngreso(TipoIngreso.SWAT)
        vm.cambiarNombre("ana")
        var guardado = false
        vm.guardar { guardado = true }
        advanceUntilIdle()
        assertFalse(guardado)
        assertEquals("La cédula es obligatoria", vm.error)
    }

    @Test
    fun `tipos sin personal de ruta lo desmarcan`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        vm.cambiarPersonalRuta(true)
        vm.cambiarTipoIngreso(TipoIngreso.SWAT)
        assertFalse(vm.personalRuta)
        assertFalse(vm.muestraPersonalRuta)
    }

    @Test
    fun `mascara de fecha inserta los guiones`() {
        assertEquals("03-08-2027", formatearFechaDDMMYYYY("03082027"))
        assertEquals("03-0", formatearFechaDDMMYYYY("030"))
    }

    @Test
    fun `guardar crea el contratista y avisa`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        advanceUntilIdle()
        vm.aplicarDocumentoEscaneado(praind("Sodexo", FechaDocumento(3, 8, 2099)))
        var guardado = false
        vm.guardar { guardado = true }
        advanceUntilIdle()
        assertTrue(guardado)
        assertNull(vm.error)
        assertFalse(vm.enviando)
        assertEquals(
            "ANA ROJAS VEGA",
            nucleo.buscarContratistas("701000000").single().nombre,
        )
    }
}
