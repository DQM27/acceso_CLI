package com.brisas.controlacceso

import java.io.File
import java.io.IOException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.ManifiestoActualizacion

/// Las reglas del manifiesto (validación, novedad, obligatoria) se prueban en
/// `mobile/rust-core/src/actualizacion.rs`. Acá sólo cómo el ViewModel mueve
/// el estado con lo que le devuelven el núcleo y la fuente.
@OptIn(ExperimentalCoroutinesApi::class)
class ActualizacionViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private var ahoraMs = 1_000_000L

    @Before
    fun preparar() = Dispatchers.setMain(dispatcher)

    @After
    fun limpiar() = Dispatchers.resetMain()

    private class FuenteFalsa(
        var manifiesto: String = manifiestoJson(),
        var instalada: Long = 32,
        var permiso: Boolean = true,
        var falloRed: Boolean = false,
        var falloApk: ApkRechazado? = null,
        override val urlManifiesto: String = URL,
    ) : FuenteActualizaciones {
        var lecturas = 0
        var instalados = 0

        override fun versionCodeInstalada() = instalada
        override fun puedeInstalar() = permiso
        override fun leerManifiesto(): String {
            lecturas++
            if (falloRed) throw IOException("sin red")
            return manifiesto
        }
        override fun descargarApk(manifiesto: ManifiestoActualizacion, alAvanzar: (Long) -> Unit): File {
            if (falloRed) throw IOException("sin red")
            alAvanzar(manifiesto.tamanoBytes / 2)
            return File("lattis.apk")
        }
        override fun verificarApk(apk: File, manifiesto: ManifiestoActualizacion) {
            falloApk?.let { throw it }
        }
        override fun instalar(apk: File) {
            instalados++
        }
    }

    private fun viewModel(fuente: FuenteActualizaciones) =
        ActualizacionViewModel(fuente, dispatcherIO = dispatcher, relojMs = { ahoraMs })

    @Test
    fun `version nueva queda disponible`() = runTest(dispatcher) {
        val vm = viewModel(FuenteFalsa())
        vm.revisar()
        advanceUntilIdle()
        val estado = vm.estado.value as EstadoActualizacion.Disponible
        assertEquals("1.4.11", estado.novedad.manifiesto.versionName)
        assertEquals(false, estado.novedad.obligatoria)
    }

    @Test
    fun `al dia no muestra aviso`() = runTest(dispatcher) {
        val vm = viewModel(FuenteFalsa(instalada = 33))
        vm.revisar()
        advanceUntilIdle()
        assertEquals(EstadoActualizacion.SinAviso, vm.estado.value)
    }

    @Test
    fun `sin red o con manifiesto invalido no avisa nada`() = runTest(dispatcher) {
        val sinRed = viewModel(FuenteFalsa(falloRed = true))
        sinRed.revisar()
        val invalido = viewModel(FuenteFalsa(manifiesto = "{}"))
        invalido.revisar()
        advanceUntilIdle()
        assertEquals(EstadoActualizacion.SinAviso, sinRed.estado.value)
        assertEquals(EstadoActualizacion.SinAviso, invalido.estado.value)
    }

    @Test
    fun `build sin actualizador no consulta`() = runTest(dispatcher) {
        val fuente = FuenteFalsa(urlManifiesto = "")
        viewModel(fuente).revisar()
        advanceUntilIdle()
        assertEquals(0, fuente.lecturas)
    }

    @Test
    fun `consulta como mucho una vez por hora`() = runTest(dispatcher) {
        val fuente = FuenteFalsa()
        val vm = viewModel(fuente)
        vm.revisar()
        advanceUntilIdle()
        ahoraMs += 59 * 60 * 1000L
        vm.revisar()
        advanceUntilIdle()
        assertEquals(1, fuente.lecturas)
        ahoraMs += 2 * 60 * 1000L
        vm.revisar()
        advanceUntilIdle()
        assertEquals(2, fuente.lecturas)
    }

    @Test
    fun `la opcional se pospone y la obligatoria no`() = runTest(dispatcher) {
        val opcional = viewModel(FuenteFalsa())
        opcional.revisar()
        val obligatoria = viewModel(FuenteFalsa(manifiesto = manifiestoJson(versionMinimaCode = 33)))
        obligatoria.revisar()
        advanceUntilIdle()
        opcional.posponer()
        obligatoria.posponer()
        // Pospuesta: la franja se oculta pero la versión sigue pendiente.
        assertTrue((opcional.estado.value as EstadoActualizacion.Disponible).pospuesta)
        val sinPosponer = obligatoria.estado.value as EstadoActualizacion.Disponible
        assertTrue(sinPosponer.novedad.obligatoria)
        assertEquals(false, sinPosponer.pospuesta)
    }

    @Test
    fun `lo pospuesto no vuelve en la siguiente revision de la misma version`() = runTest(dispatcher) {
        val vm = viewModel(FuenteFalsa())
        vm.revisar()
        advanceUntilIdle()
        vm.posponer()
        ahoraMs += 2 * 60 * 60 * 1000L
        vm.revisar()
        advanceUntilIdle()
        assertTrue((vm.estado.value as EstadoActualizacion.Disponible).pospuesta)
    }

    @Test
    fun `una version mas nueva vuelve a mostrar la franja`() = runTest(dispatcher) {
        val fuente = FuenteFalsa()
        val vm = viewModel(fuente)
        vm.revisar()
        advanceUntilIdle()
        vm.posponer()
        fuente.manifiesto = manifiestoJson(versionCode = 34, versionName = "1.4.12")
        ahoraMs += 2 * 60 * 60 * 1000L
        vm.revisar()
        advanceUntilIdle()
        val estado = vm.estado.value as EstadoActualizacion.Disponible
        assertEquals("1.4.12", estado.novedad.manifiesto.versionName)
        assertEquals(false, estado.pospuesta)
    }

    @Test
    fun `buscar ahora sin novedad deja al dia y se olvida al cerrar el menu`() = runTest(dispatcher) {
        val vm = viewModel(FuenteFalsa(instalada = 33))
        vm.buscarAhora()
        advanceUntilIdle()
        assertEquals(EstadoBusqueda.AL_DIA, vm.busqueda.value)
        vm.olvidarBusqueda()
        assertEquals(EstadoBusqueda.INACTIVA, vm.busqueda.value)
    }

    @Test
    fun `buscar ahora con novedad la muestra aunque se hubiera pospuesto`() = runTest(dispatcher) {
        val vm = viewModel(FuenteFalsa())
        vm.revisar()
        advanceUntilIdle()
        vm.posponer()
        vm.buscarAhora()
        advanceUntilIdle()
        assertEquals(false, (vm.estado.value as EstadoActualizacion.Disponible).pospuesta)
        assertEquals(EstadoBusqueda.INACTIVA, vm.busqueda.value)
    }

    @Test
    fun `buscar ahora sin red avisa que no se pudo revisar`() = runTest(dispatcher) {
        val vm = viewModel(FuenteFalsa(falloRed = true))
        vm.buscarAhora()
        advanceUntilIdle()
        assertEquals(EstadoBusqueda.SIN_RESPUESTA, vm.busqueda.value)
        assertEquals(EstadoActualizacion.SinAviso, vm.estado.value)
    }

    @Test
    fun `buscar ahora no espera la hora entre revisiones`() = runTest(dispatcher) {
        val fuente = FuenteFalsa()
        val vm = viewModel(fuente)
        vm.revisar()
        advanceUntilIdle()
        vm.buscarAhora()
        advanceUntilIdle()
        assertEquals(2, fuente.lecturas)
    }

    @Test
    fun `build sin actualizador no busca ni ofrece el boton`() = runTest(dispatcher) {
        val fuente = FuenteFalsa(urlManifiesto = "")
        val vm = viewModel(fuente)
        vm.buscarAhora()
        advanceUntilIdle()
        assertEquals(false, vm.activo)
        assertEquals(0, fuente.lecturas)
        assertEquals(EstadoBusqueda.INACTIVA, vm.busqueda.value)
    }

    @Test
    fun `sin permiso pide habilitarlo antes de descargar`() = runTest(dispatcher) {
        val fuente = FuenteFalsa(permiso = false)
        val vm = viewModel(fuente)
        vm.revisar()
        advanceUntilIdle()
        vm.actualizar()
        advanceUntilIdle()
        assertTrue(vm.estado.value is EstadoActualizacion.FaltaPermiso)
        assertEquals(0, fuente.instalados)
    }

    @Test
    fun `descarga verificada pasa al instalador`() = runTest(dispatcher) {
        val fuente = FuenteFalsa()
        val vm = viewModel(fuente)
        vm.revisar()
        advanceUntilIdle()
        vm.actualizar()
        advanceUntilIdle()
        assertTrue(vm.estado.value is EstadoActualizacion.Instalando)
        assertEquals(1, fuente.instalados)
    }

    @Test
    fun `apk rechazado no se instala y muestra el motivo`() = runTest(dispatcher) {
        val fuente = FuenteFalsa(falloApk = ApkRechazado("El archivo descargado no está firmado por Lattis."))
        val vm = viewModel(fuente)
        vm.revisar()
        advanceUntilIdle()
        vm.actualizar()
        advanceUntilIdle()
        assertEquals("El archivo descargado no está firmado por Lattis.", (vm.estado.value as EstadoActualizacion.Fallo).mensaje)
        assertEquals(0, fuente.instalados)
    }

    private companion object {
        const val URL = "https://lt.megabrisas.com/android/latest.json"

        fun manifiestoJson(versionMinimaCode: Long = 0, versionCode: Long = 33, versionName: String = "1.4.11") = """
            {"esquema":1,"version_code":$versionCode,"version_name":"$versionName",
             "url":"https://lt.megabrisas.com/android/Lattis-mobile-v$versionName-0123456789ab.apk",
             "sha256":"${"a".repeat(64)}","tamano_bytes":50000000,"version_minima_code":$versionMinimaCode}
        """.trimIndent()
    }
}
