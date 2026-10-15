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
 * significa cada nivel (12 h, 14 h) lo decide el núcleo, no esta función. */
export function filtrarPorNivel<T extends { nivel_permanencia: string }>(
  filas: readonly T[],
  nivel: FiltroPermanencia | null,
): T[] {
  return nivel === null ? [...filas] : filas.filter((fila) => fila.nivel_permanencia === nivel);
}
