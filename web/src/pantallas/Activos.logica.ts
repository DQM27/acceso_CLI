import type { TipoAdentro } from "../api/adentro";

/** Las clases en el orden del filtro, el mismo que escritorio (`desktop/src/
 * pantallas/Activos.logica.ts`): contratistas, proveedores, por correo y KOF. */
export const CLASES: TipoAdentro[] = ["CONTRATISTA", "PROVEEDOR", "POR_CORREO", "PROVISIONAL_KOF"];

/** Clase por la que se filtra la tabla: todas, o una sola. */
export type FiltroClase = "todas" | TipoAdentro;

/** Nombre de cada clase en el filtro (al pasar el mouse), como en escritorio. */
export const ETIQUETA_CLASE: Record<TipoAdentro, string> = {
  CONTRATISTA: "Contratistas",
  PROVEEDOR: "Proveedores",
  POR_CORREO: "Por correo",
  PROVISIONAL_KOF: "KOF",
};

export function filtrarPorClase<T extends { tipo: TipoAdentro }>(filas: readonly T[], filtro: FiltroClase): T[] {
  return filtro === "todas" ? [...filas] : filas.filter((fila) => fila.tipo === filtro);
}

/** Cuántas filas hay de cada clase -- los números del filtro. Cuenta todas las
 * filas, sin aplicar el filtro de gafete: son los totales de cada clase. */
export function contarPorClase<T extends { tipo: TipoAdentro }>(filas: readonly T[]): Record<TipoAdentro, number> {
  const conteo: Record<TipoAdentro, number> = { CONTRATISTA: 0, PROVEEDOR: 0, POR_CORREO: 0, PROVISIONAL_KOF: 0 };
  for (const fila of filas) conteo[fila.tipo] += 1;
  return conteo;
}

/** Filtro por gafete: sólo quienes entraron sin gafete ("S/G", `gafete_numero`
 * nulo), sólo quienes tienen uno, o todos. Sólo tiene sentido en contratistas:
 * proveedores, correo y KOF lo llevan siempre. */
export type FiltroGafete = "todos" | "con" | "sin";

export function filtrarPorGafete<T extends { gafete_numero: number | null }>(
  filas: readonly T[],
  filtro: FiltroGafete,
): T[] {
  if (filtro === "todos") return [...filas];
  const sinGafete = filtro === "sin";
  return filas.filter((fila) => (fila.gafete_numero == null) === sinGafete);
}

/** Cuántos tienen gafete y cuántos no (S/G) -- los números del filtro. */
export function contarPorGafete<T extends { gafete_numero: number | null }>(
  filas: readonly T[],
): { con: number; sin: number } {
  const sin = filtrarPorGafete(filas, "sin").length;
  return { con: filas.length - sin, sin };
}
