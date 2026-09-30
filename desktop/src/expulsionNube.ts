/**
 * Aviso en vivo `dispositivo_expulsado` (ver la migración
 * `revocacion_efectiva_dispositivos`): el servidor lo manda por el canal
 * del sitio cuando un dispositivo se retira en el panel. Todos los equipos
 * del sitio lo reciben; esto decide si es para ESTE equipo.
 */
export function esExpulsionDeEsteEquipo(aviso: unknown, dispositivoId: string): boolean {
  if (!aviso || typeof aviso !== "object") return false;
  return (aviso as Record<string, unknown>).dispositivo_id === dispositivoId;
}

/** Texto para la persona que está frente al equipo. */
export const MENSAJE_EXPULSION =
  "Este equipo fue retirado en el panel y ya no sincroniza. El trabajo local sigue disponible; " +
  "para volver a usar la nube hay que registrarlo como dispositivo nuevo.";
