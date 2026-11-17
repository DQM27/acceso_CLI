package com.brisas.controlacceso

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.CargaCarnePraind
import uniffi.control_acceso_mobile.CarnePraind
import uniffi.control_acceso_mobile.EstadoCarnePraind
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.PermisosCarnePraind

/// Guardar el carné, bajarlo sólo si hace falta y el aviso de "conectate a
/// internet" se prueban en Rust (`control_acceso::nube::praind`). Acá sólo
/// cómo el ViewModel mueve el estado.
@OptIn(ExperimentalCoroutinesApi::class)
class CarnePraindViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun preparar() = Dispatchers.setMain(dispatcher)

    @After
    fun limpiar() = Dispatchers.resetMain()

    private class FuenteFalsa(
        val conCarne: MutableSet<Long> = mutableSetOf(1L),
        val error: String? = null,
        val puedeCargar: Boolean = true,
        val sinConexion: Boolean = false,
        val nombreEnOrigen: String? = "BETO SOLIS",
        val coincide: Boolean = true,
        val errorCarga: String? = null,
    ) : FuenteCarnes {
        var consultasPermisos = 0
        val guardados = mutableListOf<Pair<Long, String>>()

        override fun estado(contratistaId: Long) =
            EstadoCarnePraind(tieneCarne = contratistaId in conCarne, guardado = false)

        override fun ver(contratistaId: Long): CarnePraind {
            error?.let { throw NucleoException.Rechazado(it) }
            return CarnePraind(tipoMime = "image/png", bytes = byteArrayOf(1, 2, 3))
        }

        override fun permisos(): PermisosCarnePraind {
            consultasPermisos++
            return PermisosCarnePraind(puedeCargar = puedeCargar && !sinConexion, sinConexion = sinConexion)
        }

        override fun cargarDesdeQr(contratistaId: Long, enlace: String, soloPrevisualizar: Boolean): CargaCarnePraind {
            errorCarga?.let { throw NucleoException.Rechazado(it) }
            if (!soloPrevisualizar) {
                guardados += contratistaId to enlace
                conCarne += contratistaId
            }
            return CargaCarnePraind(
                clave = if (soloPrevisualizar) null else "clave",
                nombreEnOrigen = nombreEnOrigen,
                nombreCoincide = coincide,
            )
        }
    }

    private val enlace = "https://prd.pw/abc"

    @Test
    fun ofreceCargarSoloSiLaNubeLoPermiteYPreguntaUnaVez() = runTest(dispatcher) {
        val fuente = FuenteFalsa(conCarne = mutableSetOf())
        val vm = CarnePraindViewModel(fuente, dispatcher)
        vm.consultar(2L)
        advanceUntilIdle()
        assertTrue(vm.puedeCargar.value)
        vm.consultar(3L)
        advanceUntilIdle()
        assertEquals(1, fuente.consultasPermisos)

        val sinPermiso = CarnePraindViewModel(FuenteFalsa(puedeCargar = false), dispatcher)
        sinPermiso.consultar(2L)
        advanceUntilIdle()
        assertFalse(sinPermiso.puedeCargar.value)
    }

    @Test
    fun sinConexionVuelveAPreguntarLosPermisos() = runTest(dispatcher) {
        val fuente = FuenteFalsa(sinConexion = true)
        val vm = CarnePraindViewModel(fuente, dispatcher)
        vm.consultar(2L)
        advanceUntilIdle()
        vm.consultar(3L)
        advanceUntilIdle()
        assertFalse(vm.puedeCargar.value)
        assertEquals(2, fuente.consultasPermisos)
    }

    @Test
    fun previsualizarNoGuardaYGuardarSi() = runTest(dispatcher) {
        val fuente = FuenteFalsa(conCarne = mutableSetOf())
        val vm = CarnePraindViewModel(fuente, dispatcher)
        vm.consultar(2L)
        advanceUntilIdle()
        vm.previsualizar(2L, "BETO SOLIS", enlace)
        assertEquals(CargaCarne.Buscando("BETO SOLIS"), vm.carga.value)
        advanceUntilIdle()
        assertEquals(CargaCarne.Previa(2L, "BETO SOLIS", enlace, "BETO SOLIS", true), vm.carga.value)
        assertTrue(fuente.guardados.isEmpty())

        vm.guardar()
        advanceUntilIdle()
        assertEquals(listOf(2L to enlace), fuente.guardados)
        assertEquals(CargaCarne.Inactiva, vm.carga.value)
        // La hoja pasa a ofrecer "Ver gafete PRAIND" y avisa que se guardó.
        assertTrue(vm.disponible.value)
        assertEquals("Gafete PRAIND de BETO SOLIS guardado.", vm.guardado.value)
        vm.avisoGuardadoVisto()
        assertEquals(null, vm.guardado.value)
    }

    @Test
    fun siNoCoincideLoDiceYCancelarNoGuarda() = runTest(dispatcher) {
        val fuente = FuenteFalsa(nombreEnOrigen = "OTRA PERSONA", coincide = false)
        val vm = CarnePraindViewModel(fuente, dispatcher)
        vm.previsualizar(2L, "BETO SOLIS", enlace)
        advanceUntilIdle()
        val previa = vm.carga.value as CargaCarne.Previa
        assertFalse(previa.coincide)
        assertEquals(
            "El gafete PRAIND es de «OTRA PERSONA», que no coincide con BETO SOLIS. Revise que sea el gafete correcto.",
            textoPrevia(previa),
        )
        vm.cancelarCarga()
        assertEquals(CargaCarne.Inactiva, vm.carga.value)
        assertTrue(fuente.guardados.isEmpty())
    }

    @Test
    fun unRechazoDeLaNubeSeMuestra() = runTest(dispatcher) {
        val aviso = "El enlace no es de un gafete PRAIND."
        val vm = CarnePraindViewModel(FuenteFalsa(errorCarga = aviso), dispatcher)
        vm.previsualizar(2L, "BETO SOLIS", enlace)
        advanceUntilIdle()
        assertEquals(CargaCarne.Error("BETO SOLIS", aviso), vm.carga.value)
    }

    @Test
    fun laCamaraSoloAceptaEnlacesWeb() {
        assertEquals(enlace, EnlaceQrPraind.desdeQr("  $enlace "))
        assertEquals("http://prd.pw/x", EnlaceQrPraind.desdeQr("http://prd.pw/x"))
        assertEquals(null, EnlaceQrPraind.desdeQr("brisas-acceso://vincular?codigo=ABC"))
        assertEquals(null, EnlaceQrPraind.desdeQr("117380396"))
        assertEquals(null, EnlaceQrPraind.desdeQr("https://"))
    }

    @Test
    fun ofreceVerCarneSoloSiElContratistaTiene() = runTest(dispatcher) {
        val vm = CarnePraindViewModel(FuenteFalsa(), dispatcher)
        vm.consultar(1L)
        advanceUntilIdle()
        assertTrue(vm.disponible.value)
        vm.consultar(2L)
        advanceUntilIdle()
        assertFalse(vm.disponible.value)
        vm.consultar(null)
        advanceUntilIdle()
        assertFalse(vm.disponible.value)
    }

    @Test
    fun abrirMuestraLaImagenYCerrarLaQuita() = runTest(dispatcher) {
        val vm = CarnePraindViewModel(FuenteFalsa(), dispatcher)
        vm.abrir(1L, "ANA PEREZ")
        assertEquals(VistaCarne.Cargando("ANA PEREZ"), vm.vista.value)
        advanceUntilIdle()
        val vista = vm.vista.value as VistaCarne.Imagen
        assertArrayEquals(byteArrayOf(1, 2, 3), vista.bytes)
        vm.cerrar()
        assertEquals(VistaCarne.Cerrada, vm.vista.value)
    }

    @Test
    fun sinInternetMuestraElAvisoDelNucleo() = runTest(dispatcher) {
        val aviso = "Para ver este gafete PRAIND por primera vez, conectate a internet."
        val vm = CarnePraindViewModel(FuenteFalsa(error = aviso), dispatcher)
        vm.abrir(1L, "ANA PEREZ")
        advanceUntilIdle()
        assertEquals(VistaCarne.Error("ANA PEREZ", aviso), vm.vista.value)
    }
}
