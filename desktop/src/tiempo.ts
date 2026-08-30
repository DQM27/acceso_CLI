/**
 * Formateo de fecha/hora compartido por las grillas que muestran movimientos
 * con timestamp (Activos, Historial, Auditoría) — estaba triplicado byte a
 * byte en las tres pantallas, mismo patrón que se sacó a `ListaFlotante.tsx`
 * antes. Mismo criterio de nombre que `src/tiempo.rs` del núcleo.
 */

/** Formato de 24 horas a propósito — sin esto `toLocaleTimeString` usa
 * AM/PM según el locale del sistema. */
export function textoHora(iso: string): string {
  return new Date(iso).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
}

/** Año-mes-día en hora LOCAL (no UTC) como string ordenable ("2026-08-28")
 * — para que el filtro/orden de columna de AG Grid funcione como texto
 * plano cronológico, sin volver a pasar por `Date` (que interpretaría
 * "2026-08-28" como medianoche UTC y podría mostrar el día anterior en un
 * huso horario negativo como Costa Rica). */
export function fechaLocalYMD(iso: string): string {
  const d = new Date(iso);
  const mes = String(d.getMonth() + 1).padStart(2, "0");
  const dia = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mes}-${dia}`;
}

export function textoFechaDDMMYYYY(ymd: string): string {
  const [anio, mes, dia] = ymd.split("-");
  return `${dia}/${mes}/${anio}`;
}

/** Año-mes-día (hora LOCAL, mismo criterio que `fechaLocalYMD`) de la fecha
 * `meses` atrás — para valores por defecto de un filtro de rango (ver
 * Historial.tsx). `hoy` es inyectable para que el test sea determinístico. */
export function fechaHaceMeses(meses: number, hoy: Date = new Date()): string {
  const d = new Date(hoy.getFullYear(), hoy.getMonth() - meses, hoy.getDate());
  const mes = String(d.getMonth() + 1).padStart(2, "0");
  const dia = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mes}-${dia}`;
}
