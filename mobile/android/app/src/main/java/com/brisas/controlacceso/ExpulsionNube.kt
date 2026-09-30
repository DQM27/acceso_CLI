package com.brisas.controlacceso

/// Aviso en vivo `dispositivo_expulsado` (ver la migración
/// `revocacion_efectiva_dispositivos`): el servidor lo manda por el canal
/// del sitio cuando un dispositivo se revoca, se suspende o se re-vincula en
/// otro equipo. Todos los equipos del sitio lo reciben; esto decide si es
/// para ESTE teléfono. Mismo criterio que `expulsionNube.ts` en escritorio.
enum class MotivoExpulsion(val mensaje: String) {
    REVOCADO(
        "Este teléfono fue dado de baja en el panel y ya no puede sincronizar. " +
            "Contactá a un administrador.",
    ),
    SUSPENDIDO(
        "Este teléfono fue suspendido en el panel. Seguís pudiendo trabajar, pero no " +
            "sincroniza hasta que lo reactiven.",
    ),
    REVINCULADO(
        "Este dispositivo se vinculó en otro equipo, así que este teléfono dejó de " +
            "sincronizar. Pedí un código nuevo en el panel si tiene que seguir operando.",
    ),
    ;

    companion object {
        /// El motivo si el aviso es para este teléfono, o `null`. Al
        /// re-vincular, el equipo nuevo comparte `dispositivo_id` con el viejo:
        /// el aviso trae la huella del que queda fuera, y sólo es para este
        /// teléfono si su huella coincide.
        fun paraEsteEquipo(
            dispositivoIdAviso: String?,
            motivo: String?,
            huellaAviso: String?,
            dispositivoId: String,
            huella: String?,
        ): MotivoExpulsion? {
            if (dispositivoIdAviso != dispositivoId) return null
            return when (motivo) {
                "revocado" -> REVOCADO
                "suspendido" -> SUSPENDIDO
                "revinculado" -> REVINCULADO.takeIf { huellaAviso == huella }
                else -> null
            }
        }
    }
}
