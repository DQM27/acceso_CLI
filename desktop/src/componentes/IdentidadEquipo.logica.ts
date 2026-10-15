import type { IdentidadEquipo } from "../api/nube";

/** Sólo la unidad operativa, en mayúsculas (`BRISAS`), para mostrarla bajo el
 * nombre de la app; `null` si todavía no llegó. */
export function textoUnidadOperativa(identidad: IdentidadEquipo | null): string | null {
  const unidad = identidad?.unidad?.trim();
  return unidad ? unidad.toLocaleUpperCase("es") : null;
}
