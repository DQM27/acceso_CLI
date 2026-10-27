package com.brisas.controlacceso

import java.time.Instant
import java.time.LocalDate
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
    val instante = instanteFechaHora(iso) ?: return "Fecha no disponible"
    val local = ZonedDateTime.ofInstant(instante, ZoneId.systemDefault())
    return local.format(FORMATO_FECHA_HORA)
}

fun instanteFechaHora(iso: String): Instant? =
    runCatching { OffsetDateTime.parse(iso).toInstant() }.getOrNull()

/// Hora de ingreso en una tarjeta de la lista: sólo la hora si fue hoy (lo
/// normal, la fecha repetía lo obvio) y día/mes delante si fue otro día, con
/// [HoraIngreso.deOtroDia] para resaltarlo: alguien que lleva adentro desde
/// ayer sí merece llamar la atención.
internal data class HoraIngreso(val texto: String, val deOtroDia: Boolean)

internal fun horaIngreso(iso: String, zona: ZoneId = ZoneId.systemDefault(), hoy: LocalDate = LocalDate.now(zona)): HoraIngreso {
    val instante = instanteFechaHora(iso) ?: return HoraIngreso("—", deOtroDia = false)
    val local = ZonedDateTime.ofInstant(instante, zona)
    return if (local.toLocalDate() == hoy) {
        HoraIngreso(local.format(FORMATO_HORA), deOtroDia = false)
    } else {
        HoraIngreso(local.format(FORMATO_DIA_HORA), deOtroDia = true)
    }
}

/// Uno solo para toda la app: armarlo interpreta el patrón, y esto corre en
/// cada fila de cada lista. Es inmutable y seguro entre hilos.
private val FORMATO_FECHA_HORA: DateTimeFormatter = DateTimeFormatter.ofPattern("dd/MM/yyyy HH:mm")
private val FORMATO_HORA: DateTimeFormatter = DateTimeFormatter.ofPattern("HH:mm")
private val FORMATO_DIA_HORA: DateTimeFormatter = DateTimeFormatter.ofPattern("dd/MM HH:mm")
