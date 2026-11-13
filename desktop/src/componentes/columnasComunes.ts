import type { ColDef } from "ag-grid-community";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

/**
 * Columnas que se repetían igual en casi todas las grillas (auditoría
 * 2026-11-13: "Gafete / S/G" en 5 pantallas, la fecha DD/MM/AAAA en 9 y la
 * hora en 15). Cada pantalla sigue eligiendo su `colId`, título y anchos con
 * `extra`: el `colId` es la clave del layout guardado de cada usuario, no se
 * puede cambiar sin que pierda sus anchos.
 */

/** Número de gafete, "S/G" si entró sin gafete. Filtro numérico ("igual a"). */
export function columnaGafete<T extends { gafete_numero?: number | null }>(
  extra: Partial<ColDef<T>> = {},
): ColDef<T> {
  return {
    field: "gafete_numero" as ColDef<T>["field"],
    type: "numero",
    headerName: "Gafete",
    flex: 0.9,
    minWidth: 90,
    valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
    ...extra,
  };
}

/**
 * Fecha de un instante ISO, mostrada DD/MM/AAAA.
 *
 * `valueGetter` (no `field`) a propósito: si la columna lee el string ISO
 * crudo, AG Grid la infiere como fecha y el filtro flotante termina siendo el
 * selector nativo de fecha+hora (`datetime-local`) en vez del propio.
 * `fechaLocalYMD` mantiene el orden cronológico como texto ("2026-08-28"
 * ordena bien sin pasar por `Date`); `valueFormatter` sólo lo reacomoda.
 */
export function columnaFecha<T>(
  colId: string,
  instante: (fila: T) => string | null | undefined,
  extra: Partial<ColDef<T>> = {},
): ColDef<T> {
  return {
    colId,
    type: "fecha",
    headerName: "Fecha",
    flex: 1.1,
    minWidth: 110,
    valueGetter: (p) => {
      const valor = p.data ? instante(p.data) : null;
      return valor ? fechaLocalYMD(valor) : "";
    },
    valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
    ...extra,
  };
}

/**
 * Hora (HH:MM) de un instante ISO; vacío si no hay (ej. todavía sin salida).
 * `valueGetter` que ya devuelve el texto, por el mismo motivo que
 * `columnaFecha`: se filtra y ordena como texto plano.
 */
export function columnaHora<T>(
  colId: string,
  instante: (fila: T) => string | null | undefined,
  extra: Partial<ColDef<T>> = {},
): ColDef<T> {
  return {
    colId,
    headerName: "Hora",
    flex: 0.9,
    minWidth: 90,
    valueGetter: (p) => {
      const valor = p.data ? instante(p.data) : null;
      return valor ? textoHora(valor) : "";
    },
    ...extra,
  };
}
