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
}
