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
export function fechaYMD(d: Date): string {
  const mes = String(d.getMonth() + 1).padStart(2, "0");
  const dia = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mes}-${dia}`;
}

export function fechaLocalYMD(iso: string): string {
  return fechaYMD(new Date(iso));
}

export function textoFechaDDMMYYYY(ymd: string): string {
  const [anio, mes, dia] = ymd.split("-");
  return `${dia}/${mes}/${anio}`;
}

/** Año-mes-día (hora LOCAL, mismo criterio que `fechaLocalYMD`) de la fecha
 * `meses` atrás — para valores por defecto de un filtro de rango (ver
 * Historial.tsx). `hoy` es inyectable para que el test sea determinístico.
 * Si el mes destino es más corto (31 de agosto − 6 meses cae en un
 * "31 de febrero" inexistente), se ajusta (clamp) al último día válido de
 * ese mes en vez de dejar que rebalse al mes siguiente — mismo criterio que
 * `subMonths` de date-fns para fechas de cara al usuario. */
export function fechaHaceMeses(meses: number, hoy: Date = new Date()): string {
  const anio = hoy.getFullYear();
  const mesDestino = hoy.getMonth() - meses;
  // Día 0 del mes SIGUIENTE al destino = último día del mes destino —
  // `Date` ya normaliza mes/año fuera de rango (negativo o >11) de forma
  // consistente entre sí, así que alcanza con este único cálculo.
  const ultimoDiaMesDestino = new Date(anio, mesDestino + 1, 0).getDate();
  const dia = Math.min(hoy.getDate(), ultimoDiaMesDestino);
  return fechaYMD(new Date(anio, mesDestino, dia));
}

/** Máscara de escritura DD/MM/AAAA (filtro de fecha de las grillas, ver
 * `FiltroFechaTabla.tsx`): deja sólo dígitos (máx. 8) y pone las barras
 * solas -- "23092026" → "23/09/2026", "2309" → "23/09". Pedido del usuario
 * 2026-09-23: fechas siempre DD/MM/AAAA, sin depender del idioma de
 * Windows (el selector nativo `type="date"` usa ese formato). */
export function mascaraFechaDDMMAAAA(texto: string): string {
  const digitos = texto.replace(/\D/g, "").slice(0, 8);
  if (digitos.length <= 2) return digitos;
  if (digitos.length <= 4) return `${digitos.slice(0, 2)}/${digitos.slice(2)}`;
  return `${digitos.slice(0, 2)}/${digitos.slice(2, 4)}/${digitos.slice(4)}`;
}

/** "23/09/2026" → medianoche LOCAL de ese día; `null` si está incompleto o
 * no existe (ej. "31/02/2026"). */
export function interpretarFechaDDMMAAAA(texto: string): Date | null {
  const partes = /^(\d{2})\/(\d{2})\/(\d{4})$/.exec(texto);
  if (!partes) return null;
  const dia = Number(partes[1]);
  const mes = Number(partes[2]);
  const anio = Number(partes[3]);
  const fecha = new Date(anio, mes - 1, dia);
  const existe =
    fecha.getFullYear() === anio && fecha.getMonth() === mes - 1 && fecha.getDate() === dia;
  return existe ? fecha : null;
}

/** `Date` → "DD/MM/AAAA" en hora local. */
export function textoFechaDDMMAAAA(fecha: Date): string {
  return textoFechaDDMMYYYY(fechaYMD(fecha));
}
