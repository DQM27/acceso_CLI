package com.brisas.controlacceso

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material.icons.filled.Search
import androidx.compose.runtime.mutableStateOf
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Test

class AccionInferiorTest {
    private fun accion(descripcion: String, alTocar: () -> Unit = {}) =
        AccionInferior(Icons.Default.Search, descripcion, mutableStateOf(alTocar))

    @Test
    fun `sin acciones el boton no se muestra`() {
        assertNull(AccionesInferiores().actual)
    }

    @Test
    fun `manda la ultima y al irse vuelve la anterior`() {
        // En una transición entre pestañas conviven la pantalla que entra y la
        // que sale: cuando la vieja se va, queda la nueva.
        val acciones = AccionesInferiores()
        val vieja = accion("Escanear documento")
        val nueva = accion("Buscar encargado")
        acciones.poner(vieja)
        acciones.poner(nueva)
        assertSame(nueva, acciones.actual)

        acciones.quitar(vieja)
        assertSame(nueva, acciones.actual)

        acciones.quitar(nueva)
        assertNull(acciones.actual)
    }

    @Test
    fun `la busqueda de la pildora sigue la misma pila que el boton`() {
        val acciones = AccionesInferiores()
        assertNull(acciones.busqueda)
        var abierta = ""
        val vieja = BusquedaInferior("Nombre o cédula para ingresar", mutableStateOf<() -> Unit>({ abierta = "vieja" }))
        val nueva = BusquedaInferior("Cédula, nombre o motivo", mutableStateOf<() -> Unit>({ abierta = "nueva" }))
        acciones.ponerBusqueda(vieja)
        acciones.ponerBusqueda(nueva)

        acciones.quitarBusqueda(vieja)
        acciones.busqueda?.abrir()

        assertEquals("nueva", abierta)
        acciones.quitarBusqueda(nueva)
        assertNull(acciones.busqueda)
    }

    @Test
    fun `tocar usa la accion vigente`() {
        var tocado = ""
        val alTocar = mutableStateOf<() -> Unit>({ tocado = "primera" })
        val accion = AccionInferior(Icons.Default.PhotoCamera, "Escanear", alTocar)

        alTocar.value = { tocado = "actualizada" }
        accion.tocar()

        assertEquals("actualizada", tocado)
    }
}
