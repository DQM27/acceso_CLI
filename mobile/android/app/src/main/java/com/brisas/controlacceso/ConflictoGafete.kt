package com.brisas.controlacceso

import uniffi.control_acceso_mobile.ConflictoGafeteActivo
import uniffi.control_acceso_mobile.TipoMovimientoGafete

/// Aviso para un movimiento con gafete que la nube rechazó porque otro equipo
/// de esta unidad ya tiene ese gafete activo. Postgres ya decidió: el
/// movimiento de ESTE teléfono es el que no quedó válido, así que el aviso lo
/// dice con esa certeza. `hora` ya viene formateada. Mismo texto que
/// `conflictoGafete.ts` en escritorio.
fun mensajeConflictoGafete(conflicto: ConflictoGafeteActivo, hora: String): String {
    val nombre = conflicto.nombre
    val gafete = conflicto.gafeteNumero
    val cierre = "no quedó registrado en la nube — otro dispositivo de este sitio ya lo tiene asignado."
    return when (conflicto.tipo) {
        TipoMovimientoGafete.PROVEEDOR ->
            "El ingreso del proveedor $nombre con gafete $gafete ($hora) $cierre"
        TipoMovimientoGafete.POR_CORREO ->
            "El ingreso por correo de $nombre con gafete de visita $gafete ($hora) $cierre"
        TipoMovimientoGafete.PROVISIONAL_KOF ->
            "El préstamo del gafete provisional $gafete a $nombre ($hora) $cierre"
        TipoMovimientoGafete.VISITA ->
            "El check-in de visita de $nombre con gafete de visita $gafete ($hora) $cierre"
        TipoMovimientoGafete.CONTRATISTA ->
            "El ingreso de $nombre con gafete $gafete ($hora) $cierre"
    }
}
