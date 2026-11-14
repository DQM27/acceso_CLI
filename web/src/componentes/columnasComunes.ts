import type { ColDef } from "ag-grid-community";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

/**
 * Columnas que se repetían en los historiales del panel (contratistas y las
 * demás clases): Gafete ("S/G"), Fecha (DD/MM/AAAA) y Hora, de entrada y de
 * salida (auditoría 2026-11-14; mismo criterio que
 * `desktop/src/componentes/columnasComunes.ts`).
 *
 * Cada pantalla sigue eligiendo su `colId`, título y anchos con `extra`. El
 * `colId` no se puede cambiar: es la clave del layout guardado de cada
 * usuario y, en las grillas con datos en el servidor, lo que el filtro por
 * columna traduce a la consulta (`historialFiltros.ts`).
 */

interface OpcionesInstante<T> extends Partial<ColDef<T>> {
  /** Texto cuando no hay instante (ej. "Activo": todavía sin salida). Sin
   * esto, la celda queda vacía. */
  sinValor?: string;
}

/** El número de gafete como se muestra y se exporta: "S/G" si entró sin gafete. */
export function textoGafete(numero: number | null | undefined): string {
  return numero == null ? "S/G" : String(numero);
}

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
    valueFormatter: (p) => textoGafete(p.value as number | null | undefined),
    ...extra,
  };
}

/**
 * Fecha de un instante ISO, mostrada DD/MM/AAAA. El valor es "AAAA-MM-DD"
 * (ordena bien como texto y es lo que espera el filtro de fecha); sin
 * instante, `sinValor`.
 */
export function columnaFecha<T>(
  colId: string,
  instante: (fila: T) => string | null | undefined,
  { sinValor = "", ...extra }: OpcionesInstante<T> = {},
): ColDef<T> {
  return {
    colId,
    type: "fecha",
    headerName: "Fecha",
    flex: 1.3,
    minWidth: 140,
    valueGetter: (p) => {
      if (!p.data) return "";
      const valor = instante(p.data);
      return valor ? fechaLocalYMD(valor) : sinValor;
    },
    valueFormatter: (p) => (!p.value || p.value === sinValor ? (p.value ?? "") : textoFechaDDMMYYYY(p.value)),
    ...extra,
  };
}

/**
 * Hora (HH:MM) de un instante ISO; sin instante, `sinValor`.
 *
 * Sin filtro propio: un filtro de texto sobre "HH:MM" era engañoso ("7"
 * también trae 17:xx y 10:07). Para acotar por tiempo se usa la columna de
 * fecha, que filtra el instante completo.
 */
export function columnaHora<T>(
  colId: string,
  instante: (fila: T) => string | null | undefined,
  { sinValor = "", ...extra }: OpcionesInstante<T> = {},
): ColDef<T> {
  return {
    colId,
    headerName: "Hora",
    flex: 1,
    minWidth: 110,
    filter: false,
    valueGetter: (p) => {
      if (!p.data) return "";
      const valor = instante(p.data);
      return valor ? textoHora(valor) : sinValor;
    },
    ...extra,
  };
}
