package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.control_acceso_mobile.ConflictoGafeteActivo
import uniffi.control_acceso_mobile.ConflictoIngresoActivo
import uniffi.control_acceso_mobile.ConflictoIngresoProveedorActivo
import uniffi.control_acceso_mobile.TipoMovimientoGafete

class FranjaConflictosTest {
    private val ingreso = ConflictoIngresoActivo(cedula = "1", contratistaNombre = "Ana Mora", sitioConflicto = "Planta Norte")
    private val proveedor = ConflictoIngresoProveedorActivo(cedula = "2", nombre = "Luis Rojas", sitioConflicto = "Bodega")
    private val correo = ConflictoIngresoProveedorActivo(cedula = "3", nombre = "Eva Solís", sitioConflicto = "Oficinas")
    private val gafete = ConflictoGafeteActivo(
        tipo = TipoMovimientoGafete.CONTRATISTA,
        nombre = "Juan Pérez",
        gafeteNumero = 12,
        fechaHora = "2026-09-30T14:00:00Z",
    )

    @Test
    fun `sin conflictos no hay mensajes`() {
        assertTrue(mensajesConflictos(emptyList(), emptyList(), emptyList(), emptyList()).isEmpty())
    }

    @Test
    fun `junta los cuatro tipos en orden y con los textos de escritorio`() {
        val mensajes = mensajesConflictos(listOf(ingreso), listOf(proveedor), listOf(correo), listOf(gafete), hora = { "08:00" })
        assertEquals(
            listOf(
                "Ana Mora tiene un ingreso activo aquí Y en Planta Norte — hay que resolverlo.",
                "Luis Rojas tiene un ingreso de proveedor activo aquí Y en Bodega — hay que resolverlo.",
                "Eva Solís tiene un ingreso por correo activo aquí Y en Oficinas — hay que resolverlo.",
                mensajeConflictoGafete(gafete, "08:00"),
            ),
            mensajes,
        )
    }

    @Test
    fun `el titulo cuenta los conflictos en singular y plural`() {
        assertEquals("1 conflicto por resolver", tituloConflictos(1))
        assertEquals("3 conflictos por resolver", tituloConflictos(3))
    }
}
