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
        assertEquals(EstadoActualizacion.SinAviso, opcional.estado.value)
        assertTrue((obligatoria.estado.value as EstadoActualizacion.Disponible).novedad.obligatoria)
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
        const val URL = "https://descargas.megabrisas.com/android/latest.json"

        fun manifiestoJson(versionMinimaCode: Long = 0) = """
            {"esquema":1,"version_code":33,"version_name":"1.4.11",
             "url":"https://descargas.megabrisas.com/android/Lattis-mobile-v1.4.11-0123456789ab.apk",
             "sha256":"${"a".repeat(64)}","tamano_bytes":50000000,"version_minima_code":$versionMinimaCode}
        """.trimIndent()
    }
}
