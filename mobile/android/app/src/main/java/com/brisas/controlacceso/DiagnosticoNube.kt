package com.brisas.controlacceso

import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.ResumenSincronizacion

/// Telemetría de diagnóstico después de cada sincronización: cómo está el
/// reloj confiable (hora del servidor + contador de arranque) y qué
/// respondió la nube a la sesión única. Sólo números, nada que identifique a
/// nadie. En los builds sin telemetría no hace nada (ni siquiera lee el
/// núcleo). Llamar fuera del hilo de UI: lee el núcleo.
fun informarDiagnosticoSincronizacion(nucleo: Nucleo, resumen: ResumenSincronizacion) {
    if (!BuildConfig.TELEMETRIA) return
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
}
