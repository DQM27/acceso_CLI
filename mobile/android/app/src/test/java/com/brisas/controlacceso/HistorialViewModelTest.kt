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
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
        val viewModel = HistorialViewModel(nucleo, dispatcherIO = dispatcher)

        advanceUntilIdle()

        assertTrue(viewModel.movimientos.isEmpty())
        assertNull(viewModel.error)
    }

    @Test
    fun `muestra el historial de otro dispositivo y filtra por cedula`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(
            archivo,
            NucleoDePrueba.sqlUsuarioRoot(),
            """
            INSERT INTO historial_sitio (uuid, sitio_id, contratista_cedula, contratista_nombre,
                hora_entrada, hora_salida, gafete_numero, dispositivo_entrada_id, actualizado_en)
            VALUES ('movimiento-remoto', 'sitio-prueba', '222222222', 'Persona de otro equipo',
                strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 hour'),
                strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), 26, 'otro-equipo',
                strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));
            """.trimIndent(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
        val viewModel = HistorialViewModel(nucleo, dispatcherIO = dispatcher)
        advanceUntilIdle()
        assertNull(viewModel.error)
        assertEquals("nube:movimiento-remoto", viewModel.movimientos.single().clave)
        assertEquals(26L, viewModel.movimientos.single().gafeteNumero)
        viewModel.cambiarTexto("222222222")
        advanceUntilIdle()
        assertEquals(1, viewModel.movimientos.size)
        viewModel.cambiarTexto("No existe")
        advanceUntilIdle()
        assertTrue(viewModel.movimientos.isEmpty())
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
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
        nucleo.registrarIngreso(1, MedioIngreso.CAMINANDO, null)

        val viewModel = HistorialViewModel(nucleo, dispatcherIO = dispatcher)
        advanceUntilIdle()

        assertEquals(1, viewModel.movimientos.size)
        assertEquals("Contratista Historial", viewModel.movimientos[0].contratistaNombre)
        assertNull(viewModel.movimientos[0].fechaHoraSalida)
    }

    @Test
    fun `ordena por fecha sin crashear con offset numerico (formato real de Rust y Supabase)`() =
        runTest(dispatcher) {
            // `to_rfc3339()` del lado Rust (y lo que devuelve Supabase para
            // `timestamptz`) usa offset numerico ("+00:00"), no el sufijo
            // "Z" que produce `strftime(...'Z')` acá arriba -- con menos de
            // dos filas, `sortedByDescending` ni siquiera llega a comparar
            // fechas, así que hacen falta DOS filas para reproducir el
            // `DateTimeParseException` real (`Instant.parse` lo rechaza,
            // `OffsetDateTime.parse` no).
            nucleo = NucleoDePrueba.abrir(
                archivo,
                NucleoDePrueba.sqlUsuarioRoot(),
                """
                INSERT INTO historial_sitio (uuid, sitio_id, contratista_cedula, contratista_nombre,
                    hora_entrada, hora_salida, gafete_numero, dispositivo_entrada_id, actualizado_en)
                VALUES
                    ('movimiento-viejo', 'sitio-prueba', '222222222', 'Persona vieja',
                        '2026-09-06T20:00:00+00:00', null, null, 'otro-equipo', '2026-09-06T20:00:00+00:00'),
                    ('movimiento-nuevo', 'sitio-prueba', '333333333', 'Persona nueva',
                        '2026-09-06T21:00:00+00:00', null, null, 'otro-equipo', '2026-09-06T21:00:00+00:00');
                """.trimIndent(),
            )
            nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
            val viewModel = HistorialViewModel(nucleo, dispatcherIO = dispatcher)
            advanceUntilIdle()

            assertNull(viewModel.error)
            assertEquals(2, viewModel.movimientos.size)
            assertEquals("Persona nueva", viewModel.movimientos[0].contratistaNombre)
            assertEquals("Persona vieja", viewModel.movimientos[1].contratistaNombre)
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
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
        nucleo.registrarIngreso(1, MedioIngreso.CAMINANDO, null)

        val viewModel = HistorialViewModel(nucleo, dispatcherIO = dispatcher)
        advanceUntilIdle()

        viewModel.cambiarTexto("no existe nadie con este nombre")
        advanceUntilIdle()

        assertTrue(viewModel.movimientos.isEmpty())
    }
}
