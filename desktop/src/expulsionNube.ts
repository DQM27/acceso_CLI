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

/** `true` si el aviso `sesion_cerrada` (el usuario inició sesión en otra
 * unidad, ver la migración `sesion_unica_por_unidad`) es para quien tiene la
 * sesión abierta en este equipo. El aviso sólo dispara una sincronización:
 * quien decide si la sesión se cierra es la nube (`nube::sesion_en_unidad`),
 * no el contenido del aviso. */
export function esCierreDeEstaSesion(aviso: unknown, cedula: string | undefined): boolean {
  if (!cedula || !aviso || typeof aviso !== "object") return false;
  return (aviso as Record<string, unknown>).cedula === cedula;
}

/** Aviso cuando la sesión se cerró porque el usuario entró en otra unidad. */
export const MENSAJE_SESION_EN_OTRA_UNIDAD =
  "Su usuario inició sesión en otra unidad: se cerró la sesión en este equipo.";

/** Texto para la persona que está frente al equipo. */
export const MENSAJE_EXPULSION =
  "Este equipo fue retirado en el panel y ya no sincroniza. El trabajo local sigue disponible; " +
  "para volver a usar la nube hay que registrarlo como dispositivo nuevo.";
