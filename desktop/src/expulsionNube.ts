import type { MotivoExpulsion } from "./api/nube";

/**
 * Aviso en vivo `dispositivo_expulsado` (ver la migración
 * `revocacion_efectiva_dispositivos`): el servidor lo manda por el canal
 * del sitio cuando un dispositivo se revoca, se suspende o se re-vincula en
 * otro equipo. Todos los equipos del sitio lo reciben; esto decide si es
 * para ESTE equipo.
 *
 * Al re-vincular, el equipo nuevo comparte `dispositivo_id` con el viejo:
 * el aviso trae la huella del que queda fuera, y sólo es para este equipo
 * si su huella coincide.
 */
export function motivoSiEsParaEsteEquipo(
  aviso: unknown,
  equipo: { dispositivo_id: string; huella: string | null },
): MotivoExpulsion | null {
  if (!aviso || typeof aviso !== "object") return null;
  const { dispositivo_id, motivo, huella } = aviso as Record<string, unknown>;
  if (dispositivo_id !== equipo.dispositivo_id) return null;
  if (motivo === "revinculado") return huella === equipo.huella ? "revinculado" : null;
  if (motivo === "revocado" || motivo === "suspendido") return motivo;
  return null;
}

/** Texto para la persona que está frente al equipo. */
export function mensajeExpulsion(motivo: MotivoExpulsion): string {
  switch (motivo) {
    case "revocado":
      return "Este equipo fue dado de baja en el panel y ya no puede sincronizar. Contactá a un administrador.";
    case "suspendido":
      return "Este equipo fue suspendido en el panel. Seguís pudiendo trabajar, pero no sincroniza hasta que lo reactiven.";
    case "revinculado":
      return "Este dispositivo se vinculó en otro equipo, así que este dejó de sincronizar. Pedí un código nuevo en el panel si tiene que seguir operando.";
  }
}
