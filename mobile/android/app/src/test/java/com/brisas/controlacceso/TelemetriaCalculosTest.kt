package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TelemetriaCalculosTest {

    // --- aJson ---

    @Test
    fun serializaMapasListasYEscalares() {
        val json = aJson(mapOf("a" to 1, "b" to listOf(true, null, 2.5), "c" to "x"))
        assertEquals("""{"a":1,"b":[true,null,2.5],"c":"x"}""", json)
    }

    @Test
    fun escapaComillasBarrasYControles() {
        assertEquals("\"a\\\"b\\\\c\\nd\\u0001\"", aJson("a\"b\\c\nd\u0001"))
    }

    @Test
    fun losNumerosNoFinitosSeVuelvenNull() {
        // `jsonb` rechaza NaN e Infinity: una fila con ellos haría fallar el lote entero.
        assertEquals("[null,null,null]", aJson(listOf(Double.NaN, Double.POSITIVE_INFINITY, Float.NaN)))
    }

    // --- percentil ---

    @Test
    fun percentilPorRangoMasCercano() {
        val valores = (1..10).map { it.toDouble() }
        assertEquals(5.0, percentil(valores, 0.5)!!, 0.0)
        assertEquals(9.0, percentil(valores, 0.9)!!, 0.0)
        assertEquals(10.0, percentil(valores, 1.0)!!, 0.0)
        assertEquals(1.0, percentil(valores, 0.0)!!, 0.0)
        assertNull(percentil(emptyList(), 0.5))
    }

    // --- calcularUsoCpu ---

    @Test
    fun usoDeCpuSobreUnNucleoYSobreElDispositivo() {
        // 1,5 s de CPU en 1 s de reloj con 8 núcleos: 150 % de un núcleo, 18,8 % del total.
        val uso = calcularUsoCpu(deltaCpuMs = 1_500, deltaParedMs = 1_000, nucleos = 8)!!
        assertEquals(150.0, uso.porcentajeUnNucleo, 0.0)
        assertEquals(18.8, uso.porcentajeDispositivo, 0.0)
    }

    @Test
    fun usoDeCpuSinIntervaloValidoEsNull() {
        assertNull(calcularUsoCpu(10, 0, 8))
        assertNull(calcularUsoCpu(10, 100, 0))
        assertNull(calcularUsoCpu(-1, 100, 8))
    }

    // --- ColaTelemetria ---

    @Test
    fun laColaDescartaLasMasViejasAlLlenarse() {
        val cola = ColaTelemetria(capacidad = 3)
        cola.agregarTodas(listOf("1", "2", "3", "4", "5"))
        assertEquals(3, cola.tamano())
        assertEquals(2L, cola.descartadas)
        assertEquals(listOf("3", "4", "5"), cola.tomar(10))
    }

    @Test
    fun unLoteDevueltoVuelveAlFrenteEnOrden() {
        val cola = ColaTelemetria(capacidad = 10)
        cola.agregarTodas(listOf("1", "2", "3", "4"))
        val lote = cola.tomar(2)
        cola.agregar("5")
        cola.devolver(lote)
        assertEquals(listOf("1", "2", "3", "4", "5"), cola.tomar(10))
    }

    @Test
    fun devolverSinLugarDescartaLasMasNuevas() {
        val cola = ColaTelemetria(capacidad = 3)
        cola.agregarTodas(listOf("1", "2"))
        val lote = cola.tomar(2)
        cola.agregarTodas(listOf("3", "4"))
        cola.devolver(lote)
        assertEquals(listOf("1", "2", "3"), cola.tomar(10))
        assertEquals(1L, cola.descartadas)
    }

    // --- AgregadorLlamadas ---

    @Test
    fun agregaLlamadasPorNombreYSeVacia() {
        val agregador = AgregadorLlamadas()
        agregador.registrar("login", 2_000_000, ok = true)
        agregador.registrar("login", 4_000_000, ok = false)
        agregador.registrar("buscar", 1_000_000, ok = true)
        val resumen = agregador.vaciar().associateBy { it["nombre"] }
        val login = resumen.getValue("login")
        assertEquals(2, login["llamadas"])
        assertEquals(1, login["errores"])
        assertEquals(2.0, login["p50_ms"])
        assertEquals(4.0, login["max_ms"])
        assertEquals(6.0, login["total_ms"])
        assertEquals(1, resumen.getValue("buscar")["llamadas"])
        assertTrue(agregador.vaciar().isEmpty())
    }

    // --- AgregadorFrames ---

    @Test
    fun cuentaFramesTrabadosYCongeladosPorPantalla() {
        val agregador = AgregadorFrames(presupuestoNanos = 16_666_667)
        listOf(10L, 12L, 40L, 800L).forEach { agregador.registrar("rutas", it * 1_000_000) }
        agregador.registrar("login", 5_000_000)
        val resumen = agregador.vaciar().associateBy { it["pantalla"] }
        val rutas = resumen.getValue("rutas")
        assertEquals(4, rutas["frames"])
        assertEquals(2, rutas["trabados"]) // 40 y 800 ms pasan de 2 × 16,7 ms
        assertEquals(1, rutas["congelados"]) // sólo 800 ms pasa de 700 ms
        assertEquals(800.0, rutas["max_ms"])
        assertEquals(0, resumen.getValue("login")["trabados"])
        assertTrue(agregador.vaciar().isEmpty())
    }

    // --- PilaPantallas ---

    @Test
    fun laPantallaActualEsLaUltimaQueEntroYNoSalio() {
        val pila = PilaPantallas()
        assertEquals(PilaPantallas.SIN_PANTALLA, pila.actual())
        pila.entrar("activos")
        pila.entrar("escanear_cedula")
        assertEquals("escanear_cedula", pila.actual())
        pila.salir("escanear_cedula")
        assertEquals("activos", pila.actual())
    }

    @Test
    fun salirFueraDeOrdenNoPierdeLaPantallaVisible() {
        // Compose puede desechar la pantalla anterior después de componer la nueva.
        val pila = PilaPantallas()
        pila.entrar("login")
        pila.entrar("principal")
        pila.salir("login")
        assertEquals("principal", pila.actual())
    }

    // --- VigilanteRetencion ---

    @Test
    fun informaUnaVezUnObjetoRetenidoMasAllaDeLaEspera() {
        var ahora = 0L
        var recolecciones = 0
        val vigilante = VigilanteRetencion(esperaMs = 10_000, reloj = { ahora }, forzarRecoleccion = { recolecciones++ })
        val retenido = Any()
        vigilante.vigilar(retenido, "EstadoCamaraOcr")

        ahora = 5_000
        assertTrue(vigilante.revisar().isEmpty())
        assertEquals(0, recolecciones)

        ahora = 12_000
        val informe = vigilante.revisar()
        assertEquals(1, informe.size)
        assertEquals("EstadoCamaraOcr", informe[0]["objeto"])
        assertEquals(12L, informe[0]["segundos_retenido"])
        assertEquals(1, recolecciones)

        ahora = 30_000
        assertTrue(vigilante.revisar().isEmpty())
        assertEquals(0, vigilante.pendientes())
        // Mantener la referencia viva hasta el final del test.
        assertTrue(retenido.hashCode() == retenido.hashCode())
    }

    // --- AgregadorViolaciones y origenEnLaApp ---

    @Test
    fun agregaViolacionesPorTipoYOrigen() {
        val agregador = AgregadorViolaciones()
        repeat(3) { agregador.registrar("DiskReadViolation", "Login.cargar:10") }
        agregador.registrar("DiskReadViolation", "Rutas.cargar:5")
        val resumen = agregador.vaciar()
        assertEquals(2, resumen.size)
        assertEquals(3, resumen.first { it["origen"] == "Login.cargar:10" }["veces"])
        assertTrue(agregador.vaciar().isEmpty())
    }

    @Test
    fun origenEsElPrimerMarcoDeLaApp() {
        val pila = arrayOf(
            StackTraceElement("android.os.StrictMode", "onReadFromDisk", "StrictMode.java", 1),
            StackTraceElement("com.brisas.controlacceso.RutasViewModel", "cargar", "RutasViewModel.kt", 42),
        )
        assertEquals("RutasViewModel.cargar:42", origenEnLaApp(pila))
        assertEquals("StrictMode.onReadFromDisk:1", origenEnLaApp(arrayOf(pila[0])))
        assertEquals("(desconocido)", origenEnLaApp(emptyArray()))
    }
}
