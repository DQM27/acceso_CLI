package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class ActualizacionCalculosTest {
    private val urlManifiesto = "https://descargas.megabrisas.com/android/latest.json"

    private fun manifiesto(
        esquema: Int = 1,
        versionCode: Long = 33,
        versionName: String = "1.4.11",
        url: String = "https://descargas.megabrisas.com/android/Lattis-mobile-v1.4.11.apk",
        sha256: String = "a".repeat(64),
        tamanoBytes: Long = 50_000_000,
        versionMinimaCode: Long = 0,
    ) = ManifiestoActualizacion(esquema, versionCode, versionName, url, sha256, tamanoBytes, versionMinimaCode)

    private fun rechaza(m: ManifiestoActualizacion) {
        assertThrows(ManifiestoInvalido::class.java) { ActualizacionCalculos.validar(m, urlManifiesto) }
    }

    @Test
    fun aceptaUnManifiestoCorrecto() {
        val m = manifiesto()
        assertEquals(m, ActualizacionCalculos.validar(m, urlManifiesto))
    }

    @Test
    fun rechazaUnEsquemaDesconocido() = rechaza(manifiesto(esquema = 2))

    @Test
    fun rechazaVersionCodeNoPositivo() = rechaza(manifiesto(versionCode = 0))

    @Test
    fun rechazaVersionNameVacio() = rechaza(manifiesto(versionName = " "))

    @Test
    fun rechazaShaMalFormado() {
        rechaza(manifiesto(sha256 = "a".repeat(63)))
        rechaza(manifiesto(sha256 = "A".repeat(64)))
        rechaza(manifiesto(sha256 = "g".repeat(64)))
    }

    @Test
    fun rechazaTamanoFueraDeRango() {
        rechaza(manifiesto(tamanoBytes = 0))
        rechaza(manifiesto(tamanoBytes = ActualizacionCalculos.TAMANO_MAXIMO_BYTES + 1))
    }

    @Test
    fun rechazaVersionMinimaMayorQueLaPublicada() {
        rechaza(manifiesto(versionCode = 33, versionMinimaCode = 34))
        rechaza(manifiesto(versionMinimaCode = -1))
    }

    @Test
    fun rechazaApkSinHttps() = rechaza(manifiesto(url = "http://descargas.megabrisas.com/android/x.apk"))

    @Test
    fun rechazaApkEnOtroServidor() {
        rechaza(manifiesto(url = "https://otro.example.com/x.apk"))
        rechaza(manifiesto(url = "https://descargas.megabrisas.com.example.com/x.apk"))
    }

    @Test
    fun rechazaUrlConCredencialesOMalFormada() {
        rechaza(manifiesto(url = "https://usuario@descargas.megabrisas.com/x.apk"))
        rechaza(manifiesto(url = "no es una url"))
    }

    @Test
    fun sinNovedadSiLaVersionPublicadaNoEsMayor() {
        assertEquals(NovedadActualizacion.AlDia, ActualizacionCalculos.novedad(33, manifiesto(versionCode = 33)))
        assertEquals(NovedadActualizacion.AlDia, ActualizacionCalculos.novedad(34, manifiesto(versionCode = 33)))
    }

    @Test
    fun novedadOpcionalSiLaInstaladaSigueSiendoCompatible() {
        val novedad = ActualizacionCalculos.novedad(32, manifiesto(versionCode = 33, versionMinimaCode = 32))
        assertEquals(NovedadActualizacion.Disponible(manifiesto(versionCode = 33, versionMinimaCode = 32), obligatoria = false), novedad)
    }

    @Test
    fun novedadObligatoriaPorDebajoDeLaVersionMinima() {
        val novedad = ActualizacionCalculos.novedad(31, manifiesto(versionCode = 33, versionMinimaCode = 32))
        assertTrue((novedad as NovedadActualizacion.Disponible).obligatoria)
    }

    @Test
    fun hexadecimalEnMinusculasConCeros() {
        assertEquals("00ff10", ActualizacionCalculos.hexadecimal(byteArrayOf(0, -1, 16)))
    }

    @Test
    fun porcentajeAcotadoYDesconocidoSinTotal() {
        assertEquals(0, ActualizacionCalculos.porcentaje(0, 200))
        assertEquals(50, ActualizacionCalculos.porcentaje(100, 200))
        assertEquals(100, ActualizacionCalculos.porcentaje(300, 200))
        assertNull(ActualizacionCalculos.porcentaje(10, 0))
    }
}
