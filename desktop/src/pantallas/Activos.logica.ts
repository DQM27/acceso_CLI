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

export const DOCE_HORAS_MS = 12 * 60 * 60 * 1000;

/** Más de 12 horas adentro desde el ingreso -- único resaltado de filas
 * que pidió el usuario (2026-09-23). `ahora` inyectable para el test. */
export function masDeDoceHoras(
  fila: { fecha_hora_ingreso: string },
  ahora: number = Date.now(),
): boolean {
  return ahora - new Date(fila.fecha_hora_ingreso).getTime() > DOCE_HORAS_MS;
}
