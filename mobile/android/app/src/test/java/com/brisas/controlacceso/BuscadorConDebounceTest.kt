package com.brisas.controlacceso

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Test

/// MV-10 (auditoría 2026-09-24): cobertura directa del helper que
/// reemplazó las 6 copias idénticas del patrón cancelar/debounce/relanzar
/// (`RutasViewModel` x3, `ProveedoresViewModel`, `GafetesProvisionalesViewModel`).
@OptIn(ExperimentalCoroutinesApi::class)
class BuscadorConDebounceTest {

    @Test
    fun `una sola busqueda pasado el debounce ejecuta la accion`() = runTest {
        val dispatcher = StandardTestDispatcher(testScheduler)
        val scope = TestScope(dispatcher)
        val buscador = BuscadorConDebounce(scope, debounceMs = 150L)
        var ejecuciones = 0

        buscador.buscar { ejecuciones++ }
        advanceTimeBy(149L)
        assertEquals(0, ejecuciones)

        advanceTimeBy(2L)
        assertEquals(1, ejecuciones)
    }

    @Test
    fun `escribir varias veces antes del debounce cancela las anteriores`() = runTest {
        val dispatcher = StandardTestDispatcher(testScheduler)
        val scope = TestScope(dispatcher)
        val buscador = BuscadorConDebounce(scope, debounceMs = 150L)
        val ejecutadas = mutableListOf<String>()

        buscador.buscar { ejecutadas.add("a") }
        advanceTimeBy(50L)
        buscador.buscar { ejecutadas.add("b") }
        advanceTimeBy(50L)
        buscador.buscar { ejecutadas.add("c") }
        advanceUntilIdle()

        // Sólo la última sobrevive -- mismo comportamiento que las 6 copias
        // que reemplaza (cancelar siempre el `Job` anterior antes de
        // relanzar).
        assertEquals(listOf("c"), ejecutadas)
    }

    @Test
    fun `inmediato salta el debounce`() = runTest {
        val dispatcher = StandardTestDispatcher(testScheduler)
        val scope = TestScope(dispatcher)
        val buscador = BuscadorConDebounce(scope, debounceMs = 150L)
        var ejecutada = false

        buscador.buscar(inmediato = true) { ejecutada = true }
        advanceUntilIdle()

        assertEquals(true, ejecutada)
    }

    @Test
    fun `cancelar sin busqueda pendiente no lanza ninguna accion`() = runTest {
        val dispatcher = StandardTestDispatcher(testScheduler)
        val scope = TestScope(dispatcher)
        val buscador = BuscadorConDebounce(scope, debounceMs = 150L)
        var ejecuciones = 0

        buscador.buscar { ejecuciones++ }
        buscador.cancelar()
        advanceUntilIdle()

        assertEquals(0, ejecuciones)
    }
}
