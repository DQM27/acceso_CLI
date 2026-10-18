import type { ClaseActivo } from "../api/activosUnificados";

/** Clase por la que se filtra la tabla: todas, o una sola (contratistas,
 * proveedores, visitas por correo o gafetes KOF). */
export type FiltroClase = "todas" | ClaseActivo;

/** Nombre de cada clase en los botones del filtro y en los mensajes. */
export const ETIQUETA_CLASE: Record<ClaseActivo, string> = {
  contratista: "Contratistas",
  proveedor: "Proveedores",
  correo: "Por correo",
  kof: "KOF",
};

/** Nombre de la clase en singular: la columna "Clase" de la tabla y de quién es
 * cada resultado en el modal de salida, donde se mezclan las cuatro. */
export const CLASE_SINGULAR: Record<ClaseActivo, string> = {
  contratista: "Contratista",
  proveedor: "Proveedor",
  correo: "Visita por correo",
  kof: "Gafete KOF",
};

export function filtrarPorClase<T extends { clase: ClaseActivo }>(
  filas: readonly T[],
  filtro: FiltroClase,
): T[] {
  return filtro === "todas" ? [...filas] : filas.filter((fila) => fila.clase === filtro);
}

/** Cuántas filas hay de cada clase -- los números de los botones. Cuenta todas
 * las filas, sin aplicar el filtro de gafete ni el de permanencia: son los
 * totales de cada clase. */
export function contarPorClase<T extends { clase: ClaseActivo }>(
  filas: readonly T[],
): Record<ClaseActivo, number> {
  const conteo: Record<ClaseActivo, number> = { contratista: 0, proveedor: 0, correo: 0, kof: 0 };
  for (const fila of filas) conteo[fila.clase] += 1;
  return conteo;
}

/** Filtro rápido por gafete (pedido del usuario 2026-09-23): ver sólo a
 * quienes entraron sin gafete ("S/G", `gafete_numero` nulo), sólo a
 * quienes tienen uno, o a todos. */
export type FiltroGafete = "todos" | "con" | "sin";

export function filtrarPorGafete<T extends { gafete_numero: number | null }>(
  filas: readonly T[],
  filtro: FiltroGafete,
): T[] {
  if (filtro === "todos") return [...filas];
  const sinGafete = filtro === "sin";
  return filas.filter((fila) => (fila.gafete_numero == null) === sinGafete);
}

/** Cuántos tienen gafete y cuántos no (S/G) -- los números de los botones del
 * filtro. El total de "todos" no se calcula acá: lo da el listado. */
export function contarPorGafete<T extends { gafete_numero: number | null }>(
  filas: readonly T[],
): { con: number; sin: number } {
  const sin = filtrarPorGafete(filas, "sin").length;
  return { con: filas.length - sin, sin };
}

/** Nivel por el que se filtra la tabla al pulsar un aviso de la barra de estado. */
export type FiltroPermanencia = "Aviso" | "Critico";

/** Deja sólo a quienes el núcleo marcó con ese nivel de permanencia. Qué
 * significa cada nivel (12 h, 14 h) lo decide el núcleo, no esta función. Las
 * filas sin nivel (todo lo que no es contratista) no coinciden con ningún nivel:
 * con un aviso pulsado sólo quedan contratistas, igual que cuenta la barra. */
export function filtrarPorNivel<T extends { nivel_permanencia: string | null }>(
  filas: readonly T[],
  nivel: FiltroPermanencia | null,
): T[] {
  return nivel === null ? [...filas] : filas.filter((fila) => fila.nivel_permanencia === nivel);
}
