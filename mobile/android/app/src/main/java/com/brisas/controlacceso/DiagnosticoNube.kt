package com.brisas.controlacceso

import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.ResumenSincronizacion

/// Telemetría después de cada sincronización: cómo está el reloj confiable
/// (hora del servidor + contador de arranque) y qué respondió la nube a la
/// sesión única. Sólo números, nada que identifique a nadie. Sin el grupo
/// `sincronizacion` no hace nada (ni siquiera lee el núcleo). Llamar fuera del
/// hilo de UI: lee el núcleo.
fun informarDiagnosticoSincronizacion(nucleo: Nucleo, resumen: ResumenSincronizacion) {
    // La sincronización pudo traer una orden nueva.
    Telemetria.refrescarGrupos()
    if (!Telemetria.permite("reloj") && !Telemetria.permite("sesion_unidad")) return
    val reloj = nucleo.estadoReloj()
    Telemetria.evento(
        "reloj",
        mapOf(
            "confiable" to reloj.confiable,
            "margen_ms" to reloj.margenMs?.toLong(),
            "diferencia_equipo_ms" to reloj.diferenciaEquipoMs,
            "ancla_hace_ms" to reloj.anclaHaceMs?.toLong(),
        ),
    )
    resumen.sesionUnidad?.let { resultado ->
        Telemetria.evento(
            "sesion_unidad",
            mapOf(
                "resultado" to resultado,
                "transcurrido_ms" to resumen.sesionTranscurridoMs?.toLong(),
            ),
        )
    }
    informarFallidosBandeja(nucleo)
}

/// Cuántas filas se leen para contar, y cuántas van con detalle en el
/// evento: el sandbox acepta hasta 8 KB por evento y cada fila ocupa unos
/// 250 bytes.
private const val MAXIMO_FALLIDOS_LEIDOS = 500u
private const val MAXIMO_FALLIDOS_CON_DETALLE = 20

/// Lo último que se mandó (cantidad + la fila más reciente), para no repetir
/// el mismo reporte en cada pulso: sólo se manda al abrir la app y cuando
/// cambia. Incidente 2026-10-30: el teléfono avisaba "28 registros no se
/// pudieron subir" y no había forma de ver cuáles sin conectarlo a la PC.
@Volatile
private var ultimoReporteFallidos: String? = null

/// Manda las filas fallidas de la bandeja de salida (ver
/// `Nucleo.detalleFallidosBandeja`): tabla, operación, identificador del
/// registro, motivo, código HTTP y de Postgres, y la restricción que la
/// rechazó. Sin cédulas ni nombres.
private fun informarFallidosBandeja(nucleo: Nucleo) {
    val fallos = runCatching { nucleo.detalleFallidosBandeja(MAXIMO_FALLIDOS_LEIDOS) }
        .getOrNull() ?: return
    val huella = "${fallos.size}|${fallos.firstOrNull()?.actualizadoEn}"
    if (fallos.isEmpty() || huella == ultimoReporteFallidos) return
    ultimoReporteFallidos = huella
    Telemetria.evento(
        "cola_fallidos",
        mapOf(
            "cantidad" to fallos.size,
            "conflictos" to fallos.count { it.motivo == "conflicto" },
            "errores" to fallos.count { it.motivo == "error" },
            "filas" to fallos.take(MAXIMO_FALLIDOS_CON_DETALLE).map { fallo ->
                mapOf(
                    "entidad" to fallo.entidad,
                    "operacion" to fallo.operacion,
                    "uuid" to fallo.entidadUuid,
                    "motivo" to fallo.motivo,
                    "intentos" to fallo.intentos,
                    "creado_en" to fallo.creadoEn,
                    "actualizado_en" to fallo.actualizadoEn,
                    "http" to fallo.estadoHttp?.toInt(),
                    "codigo" to fallo.codigo,
                    "restriccion" to fallo.restriccion,
                )
            },
        ),
    )
}
