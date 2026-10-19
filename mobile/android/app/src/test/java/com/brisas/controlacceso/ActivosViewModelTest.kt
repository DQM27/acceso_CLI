package com.brisas.controlacceso

import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.MedioIngreso
import uniffi.control_acceso_mobile.ModoBusquedaActivos
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

    private fun viewModel(): ActivosViewModel =
        ActivosViewModel(nucleo, dispatcherIO = dispatcher)

    @Test
    fun `base vacia no falla y no muestra a nadie adentro`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivo, NucleoDePrueba.sqlUsuarioRoot())
        val viewModel = viewModel()

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
        val viewModel = viewModel()
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
        val viewModel = viewModel()
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
    fun `documento escaneado con coincidencia clara abre formulario de ingreso`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');",
            """
            INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
            ) VALUES ('111111111', 'Contratista Escaneado', 1, 'SWAT', 0, 1);
            """.trimIndent(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        val viewModel = viewModel()
        advanceUntilIdle()

        viewModel.usarDocumentoEscaneadoIngreso(
            DocumentoDetectado(TipoDocumento.CEDULA_NACIONAL, "111111111"),
        )
        advanceUntilIdle()

        val seleccion = viewModel.seleccionIngreso
        assertTrue(seleccion is SeleccionIngreso.Formulario)
        assertEquals("Contratista Escaneado", (seleccion as SeleccionIngreso.Formulario).preparacion.nombre)
    }

    @Test
    fun `salida por gafete escaneado registra y mantiene modo gafete`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');",
            """
            INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso,
                fecha_vencimiento_praind
            ) VALUES ('111111117', 'Con Gafete Siete', 1, 'PRAIND', 0, 1, '2099-12-31');
            """.trimIndent(),
            "INSERT INTO gafetes (numero, tipo, estado) VALUES (7, 'CONTRATISTA', 'DISPONIBLE');",
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA)
        nucleo.registrarIngreso(1, MedioIngreso.CAMINANDO, 7L, null)
        val viewModel = viewModel()
        advanceUntilIdle()

        viewModel.registrarSalidaPorGafeteEscaneado(
            DocumentoDetectado(TipoDocumento.GAFETE_CONTRATISTA, "7", textoBusqueda = "7"),
        )
        advanceUntilIdle()

        assertEquals(ModoBusqueda.SALIDA_GAFETE, viewModel.modo)
        assertEquals("Salida registrada: Con Gafete Siete", viewModel.mensaje)
        assertTrue(nucleo.listarIngresosActivos("", ModoBusquedaActivos.NOMBRE_CEDULA).isEmpty())
    }

    @Test
    fun `salida por gafete encuentra un ingreso registrado en otro dispositivo del sitio`() = runTest(dispatcher) {
        // Caso reportado: ingreso con gafete hecho en la PC del puesto de
        // control; en el teléfono sólo vive en la caché `ingresos_remotos`
        // y antes la búsqueda por número respondía "sin ingreso activo".
        nucleo = NucleoDePrueba.abrir(
            archivo,
            """
            INSERT INTO ingresos_remotos (
                uuid, sitio_id, contratista_nombre, hora_entrada, dispositivo_entrada_id,
                actualizado_en, empresa_nombre, gafete_numero
            ) VALUES (
                'uuid-pc', 'sitio', 'Ingresado En PC', '2026-09-30T12:00:00Z', 'pc',
                '2026-09-30T12:00:00Z', 'Empresa PC', 25
            );
            """.trimIndent(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA)
        val viewModel = viewModel()
        advanceUntilIdle()
        viewModel.cambiarModo(ModoBusqueda.SALIDA_GAFETE)
        advanceUntilIdle()

        viewModel.cambiarTexto("25, 30")
        advanceUntilIdle()

        val (encontrado, ausente) = viewModel.coincidenciasGafete
        val fila = encontrado.fila
        assertTrue(fila is FilaActiva.Remota)
        assertEquals("uuid-pc", (fila as FilaActiva.Remota).remoto.uuid)
        assertEquals("Empresa PC", fila.empresaNombre)
        assertEquals(30, ausente.numero)
        assertNull(ausente.fila)
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
        val viewModel = viewModel()
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
        val viewModel = viewModel()
        advanceUntilIdle()
        viewModel.cambiarTexto("algo")
        advanceUntilIdle()

        viewModel.cambiarModo(ModoBusqueda.SALIDA_GAFETE)
        advanceUntilIdle()

        assertEquals(ModoBusqueda.SALIDA_GAFETE, viewModel.modo)
        assertEquals("", viewModel.texto)
    }


    // --- Registro de ingreso desde el formulario (M2) ---

    private fun kotlinx.coroutines.test.TestScope.formularioAbierto(tipo: String): ActivosViewModel {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');",
            """
            INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso,
                fecha_vencimiento_praind
            ) VALUES ('111111111', 'Contratista Test', 1, '$tipo', 0, 1, '2099-12-31');
            """.trimIndent(),
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA)
        val viewModel = viewModel()
        advanceUntilIdle()
        viewModel.cambiarTexto("Contratista")
        advanceUntilIdle()
        viewModel.elegir(viewModel.resultadosBusqueda.single())
        advanceUntilIdle()
        assertTrue(viewModel.seleccionIngreso is SeleccionIngreso.Formulario)
        return viewModel
    }

    @Test
    fun `registrar ingreso sin gafete lo guarda y cierra el formulario`() = runTest(dispatcher) {
        val viewModel = formularioAbierto("SWAT")

        viewModel.registrarIngreso(MedioIngreso.CAMINANDO, "", "")
        advanceUntilIdle()

        assertEquals(SeleccionIngreso.Ninguna, viewModel.seleccionIngreso)
        assertNull(viewModel.errorIngreso)
        assertEquals(1, nucleo.listarIngresosActivos("", ModoBusquedaActivos.NOMBRE_CEDULA).size)
    }

    @Test
    fun `gafete requerido y vacio se registra como sin gafete`() = runTest(dispatcher) {
        // El núcleo ya no rechaza el gafete vacío: queda S/G. Que sólo pueda
        // llegar vacío con el check "Sin gafete" marcado lo garantiza la pantalla.
        val viewModel = formularioAbierto("PRAIND")

        viewModel.registrarIngreso(MedioIngreso.CAMINANDO, "  ", "")
        advanceUntilIdle()

        assertNull(viewModel.errorIngreso)
        assertEquals(SeleccionIngreso.Ninguna, viewModel.seleccionIngreso)
        assertEquals(1, nucleo.listarIngresosActivos("", ModoBusquedaActivos.NOMBRE_CEDULA).size)
    }

    @Test
    fun `con gafete inexistente lo rechaza el nucleo y no registra`() = runTest(dispatcher) {
        // Sin vincular, el núcleo aplica sólo las reglas locales (el chequeo
        // del gafete en el otro dispositivo lo prueba `application::con_nube`).
        val viewModel = formularioAbierto("PRAIND")

        viewModel.registrarIngreso(MedioIngreso.CAMINANDO, "7", "")
        advanceUntilIdle()

        assertNotNull(viewModel.errorIngreso)
        assertFalse(viewModel.registrandoIngreso)
        assertTrue(nucleo.listarIngresosActivos("", ModoBusquedaActivos.NOMBRE_CEDULA).isEmpty())
    }

    @Test
    fun `vehiculo sin placa pide la placa y cancelar limpia el error`() = runTest(dispatcher) {
        val viewModel = formularioAbierto("SWAT")

        viewModel.registrarIngreso(MedioIngreso.VEHICULO, "", " ")
        advanceUntilIdle()
        assertEquals("La placa es obligatoria cuando el ingreso es en vehículo", viewModel.errorIngreso)
        assertTrue(nucleo.listarIngresosActivos("", ModoBusquedaActivos.NOMBRE_CEDULA).isEmpty())

        viewModel.cancelarSeleccionIngreso()
        assertNull(viewModel.errorIngreso)
    }

    @Test
    fun `caminando descarta la placa tipeada y registra`() = runTest(dispatcher) {
        val viewModel = formularioAbierto("SWAT")

        viewModel.registrarIngreso(MedioIngreso.CAMINANDO, "", "ABC123")
        advanceUntilIdle()

        assertNull(viewModel.errorIngreso)
        assertEquals(1, nucleo.listarIngresosActivos("", ModoBusquedaActivos.NOMBRE_CEDULA).size)
    }

    // --- Salida deslizada con "Deshacer" ---

    /// Dos contratistas adentro, con los gafetes 7 y 8.
    private fun abrirConDosAdentro() {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');",
            """
            INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso,
                fecha_vencimiento_praind
            ) VALUES
                ('111111117', 'Con Gafete Siete', 1, 'PRAIND', 0, 1, '2099-12-31'),
                ('111111118', 'Con Gafete Ocho', 1, 'PRAIND', 0, 1, '2099-12-31');
            """.trimIndent(),
            "INSERT INTO gafetes (numero, tipo, estado) VALUES (7, 'CONTRATISTA', 'DISPONIBLE'), (8, 'CONTRATISTA', 'DISPONIBLE');",
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA)
        nucleo.registrarIngreso(1, MedioIngreso.CAMINANDO, 7L, null)
        nucleo.registrarIngreso(2, MedioIngreso.CAMINANDO, 8L, null)
    }

    private fun adentro() = nucleo.listarIngresosActivos("", ModoBusquedaActivos.NOMBRE_CEDULA)

    @Test
    fun `deslizar una salida oculta la fila pero no la registra hasta que pasa el plazo`() = runTest(dispatcher) {
        abrirConDosAdentro()
        val viewModel = viewModel()
        advanceUntilIdle()
        val fila = viewModel.activosVisibles.first { it.contratistaNombre == "Con Gafete Siete" }

        viewModel.deslizarSalida(fila)
        runCurrent()

        // Oculta y esperando, pero todavía no hay nada escrito.
        assertEquals(listOf("Con Gafete Ocho"), viewModel.activosVisibles.map { it.contratistaNombre })
        assertEquals(fila, viewModel.salidaPendiente)
        assertEquals(2, adentro().size)

        advanceTimeBy(ActivosViewModel.MS_PARA_DESHACER_SALIDA - 1)
        runCurrent()
        assertEquals(2, adentro().size)

        advanceUntilIdle()
        assertEquals(listOf("Con Gafete Ocho"), adentro().map { it.contratistaNombre })
        assertNull(viewModel.salidaPendiente)
        assertEquals(listOf("Con Gafete Ocho"), viewModel.activosVisibles.map { it.contratistaNombre })
    }

    @Test
    fun `deshacer una salida devuelve la fila y no escribe nada`() = runTest(dispatcher) {
        abrirConDosAdentro()
        val viewModel = viewModel()
        advanceUntilIdle()
        val fila = viewModel.activosVisibles.first()

        viewModel.deslizarSalida(fila)
        runCurrent()
        viewModel.deshacerSalida()
        advanceUntilIdle()

        assertNull(viewModel.salidaPendiente)
        assertEquals(2, viewModel.activosVisibles.size)
        assertEquals(2, adentro().size)
    }

    @Test
    fun `deslizar una segunda salida registra la primera en el acto`() = runTest(dispatcher) {
        abrirConDosAdentro()
        val viewModel = viewModel()
        advanceUntilIdle()
        val primera = viewModel.activosVisibles.first { it.contratistaNombre == "Con Gafete Siete" }
        val segunda = viewModel.activosVisibles.first { it.contratistaNombre == "Con Gafete Ocho" }

        viewModel.deslizarSalida(primera)
        runCurrent()
        viewModel.deslizarSalida(segunda)
        runCurrent()

        // Sin esperar el plazo: la primera ya salió y la segunda es la pendiente.
        assertEquals(listOf("Con Gafete Ocho"), adentro().map { it.contratistaNombre })
        assertEquals(segunda, viewModel.salidaPendiente)
        assertTrue(viewModel.activosVisibles.isEmpty())

        advanceUntilIdle()
        assertTrue(adentro().isEmpty())
    }

    @Test
    fun `confirmar la salida pendiente la registra sin esperar el plazo`() = runTest(dispatcher) {
        abrirConDosAdentro()
        val viewModel = viewModel()
        advanceUntilIdle()

        viewModel.deslizarSalida(viewModel.activosVisibles.first { it.contratistaNombre == "Con Gafete Siete" })
        runCurrent()
        viewModel.confirmarSalidaPendiente()
        runCurrent()

        assertNull(viewModel.salidaPendiente)
        assertEquals(listOf("Con Gafete Ocho"), adentro().map { it.contratistaNombre })
        // Sin nada pendiente, volver a llamarlo no hace nada.
        viewModel.confirmarSalidaPendiente()
        runCurrent()
        assertEquals(1, adentro().size)
    }
}
