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
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.PreviaGafeteParaAlta
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
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA)
    }

    /// Lo que sale a internet, simulado; la lectura de los datos del gafete
    /// es la del núcleo real.
    private inner class FuenteFalsa : FuenteGafeteAlta {
        var puede = true
        var previa = PreviaGafeteParaAlta("Ana Rojas Vega", byteArrayOf(1, 2, 3))
        var errorPrevia: String? = null
        var errorCarga: String? = null
        val cargados = mutableListOf<Pair<Long, String>>()
        override fun puedeCargar() = puede
        override fun previsualizar(enlace: String): PreviaGafeteParaAlta {
            errorPrevia?.let { throw NucleoException.Rechazado(it) }
            return previa
        }
        override fun datos(textoOcr: String, nombreEnOrigen: String?) =
            nucleo.datosAltaDesdeGafetePraind(textoOcr, nombreEnOrigen)
        override fun cargar(contratistaId: Long, enlace: String) {
            errorCarga?.let { throw NucleoException.Rechazado(it) }
            cargados += contratistaId to enlace
        }
    }

    private val fuente by lazy { FuenteFalsa() }

    private fun textoGafete(vence: String = "19/06/2099", induccion: String = "19/06/2098") =
        "CARNET DE INDUCCIÓN EN NORMAS DE SEGURIDAD\nNombre: Ana Rojas Vega\nNo. de cédula: 701000000\n" +
            "Empresa: Sodexo S.A.\nFecha de inducción: $induccion\nFecha de vencimiento de\ninducción: $vence"

    private fun viewModel(lector: LectorTextoImagen = LectorTextoImagen { textoGafete() }) =
        NuevoContratistaViewModel(nucleo, dispatcherIO = dispatcher, fuenteGafete = fuente, lector = lector)

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
        vm.guardar { _, _ -> guardado = true }
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
        vm.guardar { _, _ -> }
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
        vm.guardar { _, _ -> guardado = true }
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
        var guardado: Long? = null
        vm.guardar { id, _ -> guardado = id }
        advanceUntilIdle()
        assertNull(vm.error)
        assertFalse(vm.enviando)
        // Avisa con el contratista nuevo, para darle el ingreso enseguida.
        val creado = nucleo.buscarContratistas("701000000").single()
        assertEquals("ANA ROJAS VEGA", creado.nombre)
        assertEquals(creado.id, guardado)
    }

    @Test
    fun `prellenar pone una cedula en la cedula y lo demas en el nombre, una sola vez`() = runTest(dispatcher) {
        abrirConEmpresas()
        val cedula = viewModel()
        cedula.prellenar("701000000")
        assertEquals("701000000", cedula.cedula)
        assertEquals("", cedula.nombre)

        val nombre = viewModel()
        nombre.prellenar("ana rojas")
        assertEquals("ANA ROJAS", nombre.nombre)
        assertEquals("", nombre.cedula)
        // Al girar la pantalla vuelve a llamarse: no pisa lo ya corregido.
        nombre.cambiarNombre("ANA ROJAS VEGA")
        nombre.prellenar("ana rojas")
        assertEquals("ANA ROJAS VEGA", nombre.nombre)
    }

    @Test
    fun `el QR del gafete rellena todo y al guardar carga el gafete`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        advanceUntilIdle()
        assertTrue(vm.puedeLeerQr)

        vm.aplicarQrGafete(ENLACE)
        assertTrue(vm.leyendoQr)
        advanceUntilIdle()
        assertFalse(vm.leyendoQr)
        assertEquals("701000000", vm.cedula)
        assertEquals("ANA ROJAS VEGA", vm.nombre)
        assertEquals("Sodexo", vm.empresaSeleccionada?.nombre)
        assertEquals("19-06-2099", vm.fechaPraind)
        assertTrue(vm.cargaraGafete)
        assertNull(vm.error)

        var aviso: AvisoAlta? = null
        vm.guardar { _, a -> aviso = a }
        advanceUntilIdle()
        val creado = nucleo.buscarContratistas("701000000").single()
        assertEquals(listOf(creado.id to ENLACE), fuente.cargados)
        assertEquals(AvisoAlta("Gafete PRAIND de ANA ROJAS VEGA guardado.", esError = false), aviso)
    }

    @Test
    fun `un gafete del QR vencido avisa y bloquea el guardado`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel(lector = LectorTextoImagen { textoGafete(vence = "19/06/2025", induccion = "19/06/2024") })
        advanceUntilIdle()
        vm.aplicarQrGafete(ENLACE)
        advanceUntilIdle()
        assertTrue(vm.praindVencido)
        assertEquals(
            "El gafete PRAIND venció el 19/06/2025: pida a la persona el gafete actualizado.",
            vm.error,
        )
    }

    @Test
    fun `si el OCR falla queda el nombre de la pagina del QR`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel(lector = LectorTextoImagen { null })
        advanceUntilIdle()
        vm.aplicarQrGafete(ENLACE)
        advanceUntilIdle()
        assertEquals("ANA ROJAS VEGA", vm.nombre)
        assertEquals("", vm.cedula)
        assertEquals("Gafete PRAIND leído: complete lo que falta. Al guardar se carga el gafete.", vm.mensaje)
    }

    @Test
    fun `si la nube no encuentra el gafete lo dice y no deja gafete pendiente`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        advanceUntilIdle()
        fuente.errorPrevia = "Ese gafete PRAIND ya no existe en el sistema PRAIND."
        vm.aplicarQrGafete(ENLACE)
        advanceUntilIdle()
        assertEquals("Ese gafete PRAIND ya no existe en el sistema PRAIND.", vm.error)
        assertFalse(vm.cargaraGafete)
    }

    @Test
    fun `con otra cedula el gafete no se carga`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        advanceUntilIdle()
        vm.aplicarQrGafete(ENLACE)
        advanceUntilIdle()
        vm.cambiarCedula("702000000")
        assertFalse(vm.cargaraGafete)
        var aviso: AvisoAlta? = null
        vm.guardar { _, a -> aviso = a }
        advanceUntilIdle()
        assertTrue(fuente.cargados.isEmpty())
        assertEquals(true, aviso?.esError)
    }

    @Test
    fun `si falla la carga del gafete el alta queda y se avisa`() = runTest(dispatcher) {
        abrirConEmpresas()
        val vm = viewModel()
        advanceUntilIdle()
        vm.aplicarQrGafete(ENLACE)
        advanceUntilIdle()
        fuente.errorCarga = "Revise la conexión."
        var guardado: Long? = null
        var aviso: AvisoAlta? = null
        vm.guardar { id, a ->
            guardado = id
            aviso = a
        }
        advanceUntilIdle()
        assertEquals(nucleo.buscarContratistas("701000000").single().id, guardado)
        assertEquals(
            AvisoAlta(
                "Contratista guardado, pero el gafete PRAIND no se cargó: Revise la conexión. Cárguelo desde su ingreso.",
                esError = true,
            ),
            aviso,
        )
    }

    @Test
    fun `el nombre es el de la pagina del QR aunque el OCR lea otra cosa`() = runTest(dispatcher) {
        abrirConEmpresas()
        // Caso real (2026-11-21): ML Kit puso el logo "FEMSA" después de "Nombre:".
        val vm = viewModel(lector = LectorTextoImagen { textoGafete().replace("Ana Rojas Vega", "FEMSA") })
        advanceUntilIdle()
        vm.aplicarQrGafete(ENLACE)
        advanceUntilIdle()
        assertEquals("ANA ROJAS VEGA", vm.nombre)
    }

    @Test
    fun `sin permiso del panel no se ofrece el QR`() = runTest(dispatcher) {
        abrirConEmpresas()
        fuente.puede = false
        val vm = viewModel()
        advanceUntilIdle()
        assertFalse(vm.puedeLeerQr)
    }

    private companion object {
        const val ENLACE = "https://prd.pw/abc"
    }
}
