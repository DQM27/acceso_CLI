package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/// La única muestra real es la calcomanía de número de unidad (foto
/// 2026-09-15, `22906` -- fondo rojo, dígitos blancos). El formato de
/// placa viene de fuentes públicas sobre matrícula de Costa Rica, no de
/// una foto real todavía -- ver el comentario de
/// `LectorVehiculoRuta.kt` para el detalle de qué está confirmado y qué
/// no.
class LectorVehiculoRutaTest {

    @Test
    fun extraeNumeroDeUnidadDeLaCalcomaniaReal() {
        val resultado = extraerVehiculo("22906")
        assertEquals("22906", resultado?.valor)
        assertEquals(TipoVehiculoDetectado.NUMERO_UNIDAD, resultado?.tipo)
    }

    @Test
    fun extraePlacaDeCargaConGuion() {
        val resultado = extraerVehiculo("C-12345")
        assertEquals("C12345", resultado?.valor)
        assertEquals(TipoVehiculoDetectado.PLACA, resultado?.tipo)
    }

    @Test
    fun extraePlacaDeCargaConEspacioYMinuscula() {
        val resultado = extraerVehiculo("c 1234")
        assertEquals("C1234", resultado?.valor)
        assertEquals(TipoVehiculoDetectado.PLACA, resultado?.tipo)
    }

    @Test
    fun extraePlacaParticularTresLetrasTresDigitos() {
        val resultado = extraerVehiculo("BPH-485")
        assertEquals("BPH485", resultado?.valor)
        assertEquals(TipoVehiculoDetectado.PLACA, resultado?.tipo)
    }

    @Test
    fun prefierePlacaSobreNumeroDeUnidadSiHayAmbosEnElTexto() {
        // Ruido de frame: la calcomanía de unidad y la placa no están en
        // el mismo lugar del camión, pero si ML Kit llega a leer texto de
        // ambas en un mismo frame (reflejo, borde de cuadro guía muy
        // ancho), la placa gana -- es el patrón más específico.
        val texto = "22906\nC-12345"
        val resultado = extraerVehiculo(texto)
        assertEquals("C12345", resultado?.valor)
        assertEquals(TipoVehiculoDetectado.PLACA, resultado?.tipo)
    }

    @Test
    fun rechazaCorridaDeDigitosDemasiadoCorta() {
        assertNull(extraerVehiculo("123"))
    }

    @Test
    fun rechazaCorridaDeDigitosDemasiadoLarga() {
        // 7 dígitos pegados no calzan en el rango 4-6 admitido para
        // número de unidad, y tampoco tienen forma de placa.
        assertNull(extraerVehiculo("1234567"))
    }

    @Test
    fun sinNadaReconocibleNoHayResultado() {
        assertNull(extraerVehiculo("Apunte la cámara al vehículo"))
    }

    // Muestras reales del 2026-09-17 (overlay de debug de
    // PantallaEscanearVehiculoRuta, ver comentario de `REGEX_PLACA_CARGA`
    // en LectorVehiculoRuta.kt para el detalle de cada caso).

    @Test
    fun extraePlacaDeCargaConDigitosPartidosPorMLKit() {
        // Placa real `CL371931`: el prefijo apilado se leyó como `E` y los
        // dígitos salieron partidos "37 1931". La "E" + 6 dígitos se
        // restituye como "CL" (no existe prefijo "E" solo).
        val texto = "FIAT\nE37 1931\nCOSTA RICA\nCENTROAMERICA"
        val resultado = extraerVehiculo(texto)
        assertEquals("CL371931", resultado?.valor)
        assertEquals(TipoVehiculoDetectado.PLACA, resultado?.tipo)
    }

    @Test
    fun extraePlacaDeMotoConPrefijoMDetectado() {
        val texto = "CoSmCA\n947\n369\nCENTROAMERIGA\nM"
        val resultado = extraerVehiculo(texto)
        assertEquals("M947369", resultado?.valor)
        assertEquals(TipoVehiculoDetectado.PLACA, resultado?.tipo)
    }

    @Test
    fun extraePlacaDeMotoSinElPrefijoMCuandoMlKitNoLoDetecta() {
        // Segunda moto real: "807"/"ACL" salieron perfectos, pero la "M"
        // del prefijo no apareció como línea propia esta vez -- el
        // resultado queda sin ella en vez de fallar del todo.
        val texto = "09TA RICA\n807\nACL\nNOANA"
        val resultado = extraerVehiculo(texto)
        assertEquals("807ACL", resultado?.valor)
        assertEquals(TipoVehiculoDetectado.PLACA, resultado?.tipo)
    }

    @Test
    fun palabrasDeTresLetrasRotuladasNoTapanElNumeroDeUnidad() {
        // Antes "KOF"/"CRC" se leían como placa de moto (se intenta antes
        // que el número de unidad) y la calcomanía real se perdía.
        val resultado = extraerVehiculo("KOF\nCRC\n22906")
        assertEquals("22906", resultado?.valor)
        assertEquals(TipoVehiculoDetectado.NUMERO_UNIDAD, resultado?.tipo)
    }

    // --- Confusiones letra/dígito corregidas por posición (auditoría OCR 2026-09-28) ---

    @Test
    fun corrigeLetraLeidaComoDigitoEnLaParteNumericaDeUnaParticular() {
        val resultado = extraerVehiculo("BPH48S")
        assertEquals("BPH485", resultado?.valor)
        assertEquals(TipoVehiculoDetectado.PLACA, resultado?.tipo)
    }

    @Test
    fun corrigeDigitoLeidoComoLetraEnLaParteDeLetrasDeUnaParticular() {
        assertEquals("BPH485", extraerVehiculo("8PH-485")?.valor)
    }

    @Test
    fun corrigeLetraLeidaComoDigitoEnUnaPlacaDeCarga() {
        assertEquals("CL371931", extraerVehiculo("CL 37I931")?.valor)
    }

    @Test
    fun laEDelClApiladoSoloSeRestituyeConSeisDigitos() {
        // Con otra cantidad de dígitos no hay base para suponer "CL".
        assertEquals("E12345", extraerVehiculo("E 12345")?.valor)
        assertEquals("CL123456", extraerVehiculo("E-123456")?.valor)
    }

    @Test
    fun conDosCorreccionesNoSeInventaUnaPlaca() {
        // Sin dígito verificador, más de una corrección ya es adivinar.
        assertNull(extraerVehiculo("8PH48S"))
    }

    @Test
    fun ganaLaLecturaQueNoNecesitoCorreccion() {
        // Leída tal cual es una particular; como placa de carga haría falta
        // cambiar la `B` por `8` (`SG8123`), así que no debe ganar la de carga.
        assertEquals("SGB123", extraerVehiculo("SGB123")?.valor)
    }

    @Test
    fun unNumeroDeUnidadDeSeisDigitosNoSeConvierteEnPlaca() {
        // "228" como letras serían 3 correcciones ("ZZB"): sigue siendo
        // número de unidad, no una placa fabricada.
        val resultado = extraerVehiculo("228051")
        assertEquals("228051", resultado?.valor)
        assertEquals(TipoVehiculoDetectado.NUMERO_UNIDAD, resultado?.tipo)
    }
}
