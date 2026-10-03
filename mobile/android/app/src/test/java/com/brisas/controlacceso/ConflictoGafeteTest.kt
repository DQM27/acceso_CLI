package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.control_acceso_mobile.ConflictoGafeteActivo
import uniffi.control_acceso_mobile.TipoMovimientoGafete

class ConflictoGafeteTest {
    private fun conflicto(tipo: TipoMovimientoGafete) =
        ConflictoGafeteActivo(tipo = tipo, nombre = "Juan Pérez", gafeteNumero = 12, fechaHora = "2026-09-30T14:00:00Z")

    @Test
    fun `nombra el tipo de movimiento que no quedo registrado`() {
        assertEquals(
            "El ingreso de Juan Pérez con gafete 12 (08:00) no quedó registrado en la nube — " +
                "otro dispositivo de este sitio ya lo tiene asignado.",
            mensajeConflictoGafete(conflicto(TipoMovimientoGafete.CONTRATISTA), "08:00"),
        )
        assertTrue(
            mensajeConflictoGafete(conflicto(TipoMovimientoGafete.PROVEEDOR), "08:00")
                .startsWith("El ingreso del proveedor Juan Pérez con gafete 12 (08:00)"),
        )
        assertTrue(
            mensajeConflictoGafete(conflicto(TipoMovimientoGafete.PROVISIONAL_KOF), "08:00")
                .startsWith("El préstamo del gafete provisional 12 a Juan Pérez (08:00)"),
        )
        assertTrue(
            mensajeConflictoGafete(conflicto(TipoMovimientoGafete.POR_CORREO), "08:00")
                .startsWith("El ingreso por correo de Juan Pérez con gafete de visita 12 (08:00)"),
        )
    }
}
