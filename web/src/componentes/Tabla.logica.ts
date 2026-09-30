import type { ITooltipParams } from "ag-grid-community";

/** Texto del tooltip de una celda cortada ("KAREN DE LOS ANGELE…"): el texto ya
 * formateado como se ve ("S/G", "23/09/2026") si la columna tiene
 * `valueFormatter`, si no el valor crudo; `undefined` (sin tooltip) para
 * cualquier cosa que no sea texto o número. Copiado de
 * `desktop/src/componentes/Tabla.logica.ts`. */
export function textoTooltip({ valueFormatted, value }: ITooltipParams): string | undefined {
  if (typeof valueFormatted === "string" && valueFormatted !== "") return valueFormatted;
  if (typeof value === "string" && value !== "") return value;
  if (typeof value === "number") return String(value);
  return undefined;
}

/** Comparador del filtro de fecha para columnas cuyo valor es "AAAA-MM-DD"
 * (`fechaLocalYMD`, todas las columnas de fecha de la app): el valor se guarda
 * así para que ordene bien como texto; lo que se ve en pantalla es
 * DD/MM/AAAA. */
export function compararFechaYMD(filtroMedianoche: Date, valorCelda: unknown): number {
  if (typeof valorCelda !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(valorCelda)) return -1;
  const [anio, mes, dia] = valorCelda.split("-").map(Number);
  const celda = new Date(anio, mes - 1, dia).getTime();
  const filtro = filtroMedianoche.getTime();
  return celda < filtro ? -1 : celda > filtro ? 1 : 0;
}
