package com.brisas.controlacceso

import android.hardware.camera2.CameraMetadata
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class InfoCamaraTest {
    @Test
    fun nombraLosNivelesDeHardwareDeCamera2() {
        assertEquals("LEGACY", nombreNivelHardware(CameraMetadata.INFO_SUPPORTED_HARDWARE_LEVEL_LEGACY))
        assertEquals("LIMITED", nombreNivelHardware(CameraMetadata.INFO_SUPPORTED_HARDWARE_LEVEL_LIMITED))
        assertEquals("FULL", nombreNivelHardware(CameraMetadata.INFO_SUPPORTED_HARDWARE_LEVEL_FULL))
        assertEquals("LEVEL_3", nombreNivelHardware(CameraMetadata.INFO_SUPPORTED_HARDWARE_LEVEL_3))
        assertEquals("EXTERNAL", nombreNivelHardware(CameraMetadata.INFO_SUPPORTED_HARDWARE_LEVEL_EXTERNAL))
    }

    @Test
    fun unNivelSinDatoOInesperadoNoRompe() {
        assertEquals("desconocido", nombreNivelHardware(null))
        assertEquals("otro_42", nombreNivelHardware(42))
    }

    @Test
    fun elMayorTamanoEsElDeMasPixelesNoElDeMayorAncho() {
        val tamanos = listOf(1920 to 1080, 4000 to 3000, 1080 to 1920, 4160 to 2000)
        assertEquals(4000 to 3000, mayorTamano(tamanos))
    }

    @Test
    fun sinTamanosNoHayMayor() {
        assertNull(mayorTamano(emptyList()))
    }

    @Test
    fun elProductoDePixelesNoDesbordaEnSensoresGrandes() {
        // 50 MP: 8160 x 6120 = 49,9 M, y 65535 x 65535 desbordaría un Int.
        assertEquals(65535 to 65535, mayorTamano(listOf(8160 to 6120, 65535 to 65535)))
    }

    @Test
    fun laDistanciaMinimaDeEnfoqueSeConvierteDeDioptriasACentimetros() {
        assertEquals(10f, distanciaMinimaEnfoqueCm(10f)!!, 0.001f)
        assertEquals(25f, distanciaMinimaEnfoqueCm(4f)!!, 0.001f)
    }

    @Test
    fun enfoqueFijoOSinDatoNoTieneDistancia() {
        assertNull(distanciaMinimaEnfoqueCm(0f))
        assertNull(distanciaMinimaEnfoqueCm(null))
        assertNotNull(distanciaMinimaEnfoqueCm(0.5f))
    }
}
