package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ConfirmacionRecienteTest {
    @Test
    fun `mostrar deja a la vista el texto nuevo`() {
        val confirmacion = ConfirmacionReciente()
        assertNull(confirmacion.actual)

        confirmacion.mostrar("Ingreso registrado · Ana Mora")

        assertEquals("Ingreso registrado · Ana Mora", confirmacion.actual?.texto)
    }

    @Test
    fun `el mismo texto dos veces seguidas cuenta como dos confirmaciones`() {
        val confirmacion = ConfirmacionReciente()
        confirmacion.mostrar("Salida registrada · Ana Mora")
        val primera = confirmacion.actual!!.id

        confirmacion.mostrar("Salida registrada · Ana Mora")

        assertNotEquals(primera, confirmacion.actual!!.id)
    }

    @Test
    fun `ocultar la anterior no borra una nueva que llego mientras tanto`() {
        val confirmacion = ConfirmacionReciente()
        confirmacion.mostrar("Ingreso registrado · Ana Mora")
        val anterior = confirmacion.actual!!.id
        confirmacion.mostrar("Ingreso registrado · Luis Rojas")

        confirmacion.ocultar(anterior)
        assertEquals("Ingreso registrado · Luis Rojas", confirmacion.actual?.texto)

        confirmacion.ocultar(confirmacion.actual!!.id)
        assertNull(confirmacion.actual)
    }
}
