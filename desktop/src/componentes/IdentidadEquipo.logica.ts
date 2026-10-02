import type { IdentidadEquipo } from "../api/nube";

/** Texto con la unidad y la etiqueta del equipo, o `null` si todavía no
 * llegó ninguna de las dos (equipo recién actualizado, sin token nuevo):
 * mejor no mostrar nada que mostrar un dato inventado. */
export function textoIdentidadEquipo(identidad: IdentidadEquipo | null): string | null {
  const unidad = identidad?.unidad?.trim();
  const etiqueta = identidad?.etiqueta?.trim();
  if (unidad && etiqueta) return `Unidad: ${unidad} · ${etiqueta}`;
  if (unidad) return `Unidad: ${unidad}`;
  if (etiqueta) return `Equipo: ${etiqueta}`;
  return null;
}
