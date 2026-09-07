package com.brisas.controlacceso

import java.time.OffsetDateTime
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter

/// Mismo criterio que `desktop/src/tiempo.ts` (textoHora/textoFechaDDMMYYYY):
/// se muestra en la hora LOCAL del dispositivo, formato 24h — no UTC crudo.
/// `OffsetDateTime` (no `Instant.parse`) porque acepta tanto "...Z" como
/// "...+00:00"; a qué formatea exactamente `to_rfc3339()` del lado de Rust
/// no hace falta acoplarlo aquí.
fun textoFechaHora(iso: String): String {
    val instante = OffsetDateTime.parse(iso).toInstant()
    val local = ZonedDateTime.ofInstant(instante, ZoneId.systemDefault())
    return local.format(DateTimeFormatter.ofPattern("dd/MM/yyyy HH:mm"))
}
