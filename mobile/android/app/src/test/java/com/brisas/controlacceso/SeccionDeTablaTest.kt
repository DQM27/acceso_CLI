package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class SeccionDeTablaTest {

    @Test
    fun cadaTablaEnVivoRefrescaSuSeccion() {
        assertEquals(SeccionPrincipal.CONTRATISTAS, seccionDeTabla("ingresos"))
        assertEquals(SeccionPrincipal.PROVEEDORES, seccionDeTabla("ingresos_proveedor"))
        assertEquals(SeccionPrincipal.CORREO, seccionDeTabla("ingresos_correo"))
        assertEquals(SeccionPrincipal.KOF, seccionDeTabla("prestamos_gafete_provisional"))
    }

    @Test
    fun unaTablaDesconocidaNoTieneSeccion() {
        assertNull(seccionDeTabla("gafetes"))
        assertNull(seccionDeTabla(null))
    }
}
