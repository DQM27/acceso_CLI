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

export const DOCE_HORAS_MS = 12 * 60 * 60 * 1000;

/** Más de 12 horas adentro desde el ingreso -- único resaltado de filas
 * que pidió el usuario (2026-09-23). `ahora` inyectable para el test. */
export function masDeDoceHoras(
  fila: { fecha_hora_ingreso: string },
  ahora: number = Date.now(),
): boolean {
  return ahora - new Date(fila.fecha_hora_ingreso).getTime() > DOCE_HORAS_MS;
}

/** Cuántas de las filas llevan más de 12 horas adentro (el aviso de la barra
 * de estado). Misma regla que el resaltado de la tabla. */
export function contarMasDeDoceHoras(
  filas: readonly { fecha_hora_ingreso: string }[],
  ahora: number = Date.now(),
): number {
  return filas.filter((fila) => masDeDoceHoras(fila, ahora)).length;
}

/** Texto del aviso de larga permanencia, en singular o plural. */
export function textoAvisoPermanencia(cantidad: number): string {
  return cantidad === 1
    ? "1 lleva más de 12 h adentro"
    : `${cantidad} llevan más de 12 h adentro`;
}
