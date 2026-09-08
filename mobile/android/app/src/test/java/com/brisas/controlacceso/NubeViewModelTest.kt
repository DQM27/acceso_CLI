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
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.Nucleo

/// Sólo cubre lo que se puede probar sin red real -- `sincronizar()` sin
/// secreto en el store falla antes de que `Nucleo` intente hablar con la
/// nube, así que alcanza para probar las reglas de autorización por rol
/// (`Operacion::UsarNube` no es exclusivo de Root, a diferencia de la
/// vieja `Operacion::GestionarNube`). Contra Supabase real no se prueba
/// acá -- mismo motivo que `tests/nube_smoke.rs` del lado Rust
/// (`#[ignore]`, requiere red y un secreto de dispositivo real).
@OptIn(ExperimentalCoroutinesApi::class)
class NubeViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    private lateinit var archivoDb: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        Dispatchers.setMain(dispatcher)
        archivoDb = File.createTempFile("nube_test", ".db").apply { deleteOnExit() }
    }

    @After
    fun limpiar() {
        if (::nucleo.isInitialized) nucleo.close()
        archivoDb.delete()
        Dispatchers.resetMain()
    }

    @Test
    fun `sincronizar sin secreto guardado falla sin intentar red`() = runTest(dispatcher) {
        nucleo = NucleoDePrueba.abrir(archivoDb, NucleoDePrueba.sqlUsuarioRoot())
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
        val viewModel = NubeViewModel(nucleo, SecretoDispositivoStoreDePrueba(), dispatcherIO = dispatcher)

        viewModel.sincronizar()
        advanceUntilIdle()

        assertNotNull(viewModel.error)
        assertNull(viewModel.ultimoResumen)
        assertFalse(viewModel.sincronizando)
    }

    @Test
    fun `Operador puede intentar sincronizar y falla por falta de secreto, no por permiso`() =
        runTest(dispatcher) {
            nucleo = NucleoDePrueba.abrir(
                archivoDb,
                """
                INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                    '888888888', 'Operador Test', '${NucleoDePrueba.HASH_CLAVE_PRUEBA}', 'OPERADOR', 1
                );
                """.trimIndent(),
            )
            nucleo.autenticar("888888888", NucleoDePrueba.CLAVE_PRUEBA, "", "")
            val viewModel = NubeViewModel(nucleo, SecretoDispositivoStoreDePrueba(), dispatcherIO = dispatcher)

            viewModel.sincronizar()
            advanceUntilIdle()

            // Si autorizar_uso_nube rechazara al Operador, el mensaje sería
            // de autorización; acá tiene que ser el de "sin secreto" --
            // confirma que UsarNube (a diferencia de GestionarNube) no es
            // exclusivo de Root.
            val mensaje = viewModel.error
            assertNotNull(mensaje)
            assertTrue(mensaje!!.contains("secreto", ignoreCase = true))
        }
}
