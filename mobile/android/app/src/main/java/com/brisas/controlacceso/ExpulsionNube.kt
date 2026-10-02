package com.brisas.controlacceso

/// Aviso en vivo `dispositivo_expulsado` (ver la migración
/// `revocacion_efectiva_dispositivos`): el servidor lo manda por el canal
/// del sitio cuando un dispositivo se retira en el panel. Todos los equipos
/// del sitio lo reciben; esto decide si es para ESTE teléfono. Mismo
/// criterio que `expulsionNube.ts` en escritorio.
object ExpulsionNube {
    const val MENSAJE =
        "Este teléfono fue retirado en el panel y ya no sincroniza. El trabajo local sigue " +
            "disponible; para volver a usar la nube hay que registrarlo como dispositivo nuevo."

    fun esParaEsteEquipo(dispositivoIdAviso: String?, dispositivoId: String): Boolean =
        dispositivoIdAviso == dispositivoId

    /// Aviso cuando la sesión se cerró porque el usuario entró en otra unidad
    /// (sesión única por unidad, migración `sesion_unica_por_unidad`).
    const val MENSAJE_SESION_EN_OTRA_UNIDAD =
        "Su usuario inició sesión en otra unidad: se cerró la sesión en este teléfono."

    /// Aviso en vivo `sesion_cerrada`: es para este teléfono si nombra al
    /// usuario con sesión abierta. Sólo dispara una sincronización: quien
    /// decide si la sesión se cierra es la nube, no el aviso. Mismo criterio
    /// que `expulsionNube.ts` en escritorio.
    fun esCierreDeEstaSesion(cedulaAviso: String?, cedula: String): Boolean =
        cedula.isNotEmpty() && cedulaAviso == cedula
}
