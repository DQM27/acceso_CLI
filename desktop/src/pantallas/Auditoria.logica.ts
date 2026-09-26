import type { CambioAuditado } from "../api";

/** Para cada (entidad, entidad_id), el `entidad_nombre` de la fila con la
 * `fecha_hora` más reciente — `items` ya viene ordenado por fecha DESC
 * desde el núcleo, así que la primera fila vista por combinación ya es la
 * más nueva. */
export function nombresActuales(items: CambioAuditado[]): Map<string, string> {
  const nombres = new Map<string, string>();
  for (const item of items) {
    const clave = `${item.entidad}:${item.entidad_id}`;
    if (!nombres.has(clave)) nombres.set(clave, item.entidad_nombre);
  }
  return nombres;
}
