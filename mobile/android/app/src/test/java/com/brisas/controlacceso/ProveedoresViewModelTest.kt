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
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.Nucleo

/// Mismo criterio que `ActivosViewModelTest` -- un solo `StandardTestDispatcher`
/// compartido entre `Dispatchers.Main` y `dispatcherIO`.
@OptIn(ExperimentalCoroutinesApi::class)
class ProveedoresViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private lateinit var archivo: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        Dispatchers.setMain(dispatcher)
        archivo = File.createTempFile("proveedores_test", ".db").apply { deleteOnExit() }
    }

    @After
    fun limpiar() {
        if (::nucleo.isInitialized) nucleo.close()
        archivo.delete()
        Dispatchers.resetMain()
    }

    private fun viewModel(secretoStore: SecretoDispositivoStore = SecretoDispositivoStoreDePrueba()): ProveedoresViewModel =
        ProveedoresViewModel(nucleo, secretoStore = secretoStore, dispatcherIO = dispatcher)

    @Test
    fun `registrar ingreso de proveedor sigue funcionando con el chequeo cruzado agregado por MV-04`() = runTest(dispatcher) {
        // MV-04 (auditoría 2026-09-24): antes de este fix, nada en Kotlin
        // llamaba a `proveedorActivoEnOtroSitioConSecreto` -- este test
        // cubre que agregarlo (`ProveedoresViewModel.registrarIngreso`) no
        // rompe el camino feliz. Secreto vacío (no `null`, eso tiraría
        // `SecretoDispositivoNoEncontradoException` antes de llegar acá) --
        // tanto `proveedor_activo_en_otro_sitio_con_secreto` como el
        // chequeo de gafete que ya existía devuelven de inmediato
        // `None`/`Ok(false)` con secreto vacío (mobile/rust-core/src/lib.rs),
        // sin tocar la red -- un secreto NO vacío pero inválido sí la toca
        // (autenticación real contra Supabase) y no es reproducible en un
        // test unitario sin red.
        nucleo = NucleoDePrueba.abrir(
            archivo,
            "INSERT INTO gafetes (numero, tipo, estado) VALUES (7, 'PROVEEDOR', 'DISPONIBLE');",
            NucleoDePrueba.sqlUsuarioRoot(),
        )
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA, "", "")
        val viewModel = viewModel(SecretoDispositivoStoreDePrueba(secreto = ""))
        val empresaId = nucleo.crearEmpresaProveedor("Empresa Proveedora Test")
        advanceUntilIdle()

        viewModel.cambiarCedula("111222333")
        viewModel.cambiarNombre("Proveedor Test")
        viewModel.elegirEmpresa(uniffi.control_acceso_mobile.EmpresaProveedor(empresaId, "Empresa Proveedora Test", true))

        var seExecutoOnExito = false
        viewModel.registrarIngreso(gafeteNumero = 7L, onExito = { seExecutoOnExito = true })
        advanceUntilIdle()

        assertNull(viewModel.error)
        assertEquals("Ingreso registrado", viewModel.mensaje)
        assertEquals(true, seExecutoOnExito)
    }
}
