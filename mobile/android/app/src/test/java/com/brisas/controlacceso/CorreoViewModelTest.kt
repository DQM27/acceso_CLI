package com.brisas.controlacceso

import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.Nucleo

/// Contra el núcleo real, mismo criterio que `VisitasViewModelTest`.
@OptIn(ExperimentalCoroutinesApi::class)
class CorreoViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private lateinit var archivo: File
    private lateinit var nucleo: Nucleo

    @Before
    fun preparar() {
        Dispatchers.setMain(dispatcher)
        archivo = File.createTempFile("correo_test", ".db").apply { deleteOnExit() }
        nucleo = NucleoDePrueba.abrir(archivo, NucleoDePrueba.sqlUsuarioRoot())
        nucleo.autenticar("999999999", NucleoDePrueba.CLAVE_PRUEBA)
    }

    @After
    fun limpiar() {
        nucleo.close()
        archivo.delete()
        Dispatchers.resetMain()
    }

    /// Como el resto de los nombres de la app: en mayúscula al escribirlo y
    /// al leerlo del documento.
    @Test
    fun `el nombre de la visita queda en mayuscula al escribirlo y al escanearlo`() {
        val viewModel = CorreoViewModel(nucleo, dispatcherIO = dispatcher)

        viewModel.cambiarNombre("Ana Solano")
        assertEquals("ANA SOLANO", viewModel.nombre)

        viewModel.rellenarDesdeDocumento(null, "Luis", "Mora Díaz")
        assertEquals("LUIS MORA DÍAZ", viewModel.nombre)
    }
}
