/** Mismo texto que muestra el botón de `SelectorUnidadesOperativas` — se exporta
 * para no reimplementar este formateo en otro lado, mismo criterio que `textoRangoFecha`. */
export function textoUnidadesOperativas(total: number, excluidas: number): string {
  if (total === 0) return "Unidades operativas";
  if (excluidas === 0) return "Todas las unidades";
  const incluidas = total - excluidas;
  if (incluidas === 0) return "Ninguna unidad";
  return `${incluidas} de ${total} unidades`;
}
