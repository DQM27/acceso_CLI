/**
 * Aviso en vivo `telemetria_config` (ver la migración
 * `telemetria_ordenes_por_unidad`): el servidor lo manda por el canal del
 * sitio cuando el panel cambia la orden de telemetría de un equipo. Todos los
 * equipos del sitio lo reciben; esto decide si es para ESTE equipo. Quien
 * valida el contenido y decide si la orden es más nueva es el núcleo
 * (`nube::aplicar_aviso_telemetria`), no este filtro.
 */
export function esOrdenDeEsteEquipo(aviso: unknown, dispositivoId: string): boolean {
  if (!aviso || typeof aviso !== "object") return false;
  return (aviso as Record<string, unknown>).dispositivo_id === dispositivoId;
}
