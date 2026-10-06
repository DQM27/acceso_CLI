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
import uniffi.control_acceso_mobile.OrigenVisita
import uniffi.control_acceso_mobile.VerificacionVisita

/// Contra el núcleo real (sin vincular: sin chequeos de nube), mismo
/// criterio que `ProveedoresViewModelTest`.
@OptIn(ExperimentalCoroutinesApi::class)
class VisitasViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private lateinit var archivo: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        Dispatchers.setMain(dispatcher)
        archivo = File.createTempFile("visitas_test", ".db").apply { deleteOnExit() }
    }

    @After
    fun limpiar() {
        if (::nucleo.isInitialized) nucleo.close()
        archivo.delete()
        Dispatchers.resetMain()
    }

    /// Cita de "Laura Mora" para un visitante, entre `desde` y `hasta` días
    /// contados desde hoy en Costa Rica (UTC-6), el "hoy" del núcleo.
    private fun cita(id: Int, cedula: String, desde: Int, hasta: Int, placa: String? = null) = listOf(
        """INSERT INTO citas (id, uuid, motivo, fecha_desde, fecha_hasta, anfitrion_nombre,
               anfitrion_correo, estado, creado_en)
           VALUES ($id, 'uuid-cita-$id', 'Auditoría', date('now', '-6 hours', '$desde day'), date('now', '-6 hours', '$hasta day'),
               'Laura Mora', 'laura@ejemplo.com', 'VIGENTE', '2026-08-01T00:00:00Z')""",
        """INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre, placa_vehiculo)
           VALUES ($id, 'uuid-visitante-$id', $id, '$cedula', 'Carlos Rojas', ${placa?.let { "'$it'" } ?: "NULL"})""",
    )

    private fun abrir(vararg citas: List<String>): VisitasViewModel {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            "INSERT INTO gafetes (numero, tipo, estado) VALUES (5, 'VISITA', 'DISPONIBLE')",
            NucleoDePrueba.sqlUsuarioRoot(),
            *citas.flatMap { it }.toTypedArray(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA)
        return VisitasViewModel(nucleo, dispatcherIO = dispatcher)
    }

    @Test
    fun `una cedula entra con gafete y vehiculo, y la misma cedula despues sale`() = runTest(dispatcher) {
        val viewModel = abrir(cita(1, "108470293", -1, 1, placa = "ABC123"))

        viewModel.cambiarCedula("01-0847-0293")
        viewModel.verificar()
        advanceUntilIdle()
        val entrada = viewModel.verificacion as VerificacionVisita.Entrada
        assertEquals("Laura Mora", entrada.visita.anfitrion)
        // La placa que dejó el anfitrión propone "vehículo".
        assertTrue(viewModel.enVehiculo)
        assertEquals("ABC123", viewModel.placa)

        viewModel.cambiarGafete("5")
        viewModel.registrarEntrada()
        advanceUntilIdle()
        assertNull(viewModel.error)
        assertEquals("", viewModel.cedula)
        assertNull(viewModel.verificacion)
        // Queda en la lista de quién está adentro, como en contratistas.
        assertEquals(listOf("Carlos Rojas"), viewModel.adentro.map { it.nombre })
        assertTrue(viewModel.adentro.single().origen is OrigenVisita.EsteEquipo)

        // Verificar a alguien que ya está adentro abre el diálogo de salida.
        viewModel.verificar("108470293")
        advanceUntilIdle()
        assertNull(viewModel.verificacion)
        val salida = viewModel.seleccionSalida!!
        assertEquals(5L, salida.gafeteNumero)
        assertEquals("ABC123", salida.placa)

        viewModel.confirmarSalida(salida)
        advanceUntilIdle()
        assertNull(viewModel.error)
        assertNull(viewModel.seleccionSalida)
        assertTrue(viewModel.adentro.isEmpty())
    }

    @Test
    fun `una visita que entro por la PC aparece en la lista y sale desde su tarjeta`() = runTest(dispatcher) {
        val viewModel = abrir(
            cita(1, "108470293", -1, 1),
            listOf(
                """INSERT INTO movimientos_visita_remotos (uuid, sitio_id, cedula, nombre, empresa,
                       anfitrion_nombre, motivo, gafete_numero, placa, hora_entrada,
                       usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en)
                   VALUES ('uuid-pc', 's1', '108470293', 'Carlos Rojas', NULL, 'Laura Mora',
                       'Auditoría', 7, NULL, '2026-10-06T14:00:00Z', 'Guarda PC', 'pc',
                       '2026-10-06T14:00:00Z')""",
            ),
        )
        advanceUntilIdle()

        val tarjeta = viewModel.adentro.single()
        assertEquals(OrigenVisita.OtroEquipo("uuid-pc"), tarjeta.origen)
        assertEquals("Guarda PC", tarjeta.usuarioEntradaNombre)

        // Su cédula no ofrece otra entrada: abre la salida.
        viewModel.verificar("108470293")
        advanceUntilIdle()
        assertEquals(tarjeta, viewModel.seleccionSalida)
    }

    @Test
    fun `una cita de otro dia es un aviso informativo y caminando no pide placa`() = runTest(dispatcher) {
        val viewModel = abrir(cita(1, "108470293", 3, 4), cita(2, "200000002", 0, 0))

        viewModel.verificar("108470293")
        advanceUntilIdle()
        val aviso = viewModel.verificacion as VerificacionVisita.Aviso
        assertTrue(aviso.informativo)

        viewModel.limpiar()
        viewModel.verificar("200000002")
        advanceUntilIdle()
        assertEquals(false, viewModel.enVehiculo)
        viewModel.registrarEntrada()
        advanceUntilIdle()
        assertNull(viewModel.error)
        assertEquals(listOf("Carlos Rojas"), viewModel.adentro.map { it.nombre })
    }

    @Test
    fun `en vehiculo sin placa lo rechaza el nucleo con su mensaje`() = runTest(dispatcher) {
        val viewModel = abrir(cita(1, "200000002", 0, 0))
        viewModel.verificar("200000002")
        advanceUntilIdle()
        assertEquals("Sólo hoy", (viewModel.verificacion as VerificacionVisita.Entrada).visita.vigencia)
        viewModel.cambiarEnVehiculo(true)
        viewModel.registrarEntrada()
        advanceUntilIdle()
        assertEquals("Escriba la placa del vehículo", viewModel.error)
        assertTrue(viewModel.adentro.isEmpty())
        assertTrue(viewModel.verificacion is VerificacionVisita.Entrada)
    }
}
