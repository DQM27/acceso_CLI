package com.brisas.controlacceso

/// Aviso en vivo `telemetria_config` (ver la migración
/// `telemetria_ordenes_por_unidad`): el servidor lo manda por el canal del
/// sitio cuando el panel cambia la orden de telemetría de un equipo. Todos los
/// equipos del sitio lo reciben; esto decide si es para ESTE teléfono. Quien
/// valida el contenido y decide si la orden es más nueva es el núcleo
/// (`aplicarOrdenTelemetria`), no este filtro. Mismo criterio que
/// `ordenTelemetria.ts` en escritorio.
object OrdenTelemetriaNube {
    fun esParaEsteEquipo(dispositivoIdAviso: String?, dispositivoId: String): Boolean =
        dispositivoIdAviso != null && dispositivoIdAviso == dispositivoId
}
