/**
 * Formateo de fecha/hora compartido por las grillas que muestran movimientos
 * con timestamp. Copiado tal cual de `desktop/src/tiempo.ts` — mismo criterio
 * de nombre que `src/tiempo.rs` del núcleo.
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

// Costa Rica no usa horario de verano (fijo UTC-6 todo el año) -- mismo
// criterio que `ZONA_APLICACION`/`inicio_dia_costa_rica_utc` en
// `src/tiempo.rs`. La web no tiene backend propio (habla directo con
// Supabase desde el navegador, a diferencia de escritorio que resuelve esto
// en Rust antes de tocar la base), así que este cálculo tiene que vivir acá
// -- y a propósito NO usa la zona horaria del navegador (`new Date(ymd)`
// interpretaría el YMD elegido en el selector como si fuera la fecha en el
// huso horario del que está mirando el panel, no la del sitio en Costa Rica).
const OFFSET_COSTA_RICA = "-06:00";

/** Medianoche de `ymd` en Costa Rica, como instante UTC -- límite inferior
 * de un filtro "desde". */
export function inicioDiaCostaRicaUtc(ymd: string): string {
  return `${ymd}T00:00:00${OFFSET_COSTA_RICA}`;
}

/** Medianoche del día SIGUIENTE a `ymd` en Costa Rica, como instante UTC --
 * límite superior EXCLUSIVO de un filtro "hasta": comparar con `<` esto
 * (no `<=` contra el propio `ymd`) es lo que incluye el día completo en vez
 * de sólo el instante exacto de su propia medianoche. Mismo criterio que
 * `rango_utc` en `desktop/src-tauri/src/comandos/historial.rs`. La aritmética
 * de fecha se hace en UTC puro (`Date.UTC`), no con getters locales -- acá
 * sólo importa sumar un día calendario a un YMD ya fijo, sin que el huso
 * horario del navegador se meta en el resultado. */
export function inicioDiaSiguienteCostaRicaUtc(ymd: string): string {
  const [anio, mes, dia] = ymd.split("-").map(Number);
  const siguiente = new Date(Date.UTC(anio, mes - 1, dia + 1));
  const ymdSiguiente = `${siguiente.getUTCFullYear()}-${String(siguiente.getUTCMonth() + 1).padStart(2, "0")}-${String(
    siguiente.getUTCDate(),
  ).padStart(2, "0")}`;
  return inicioDiaCostaRicaUtc(ymdSiguiente);
}

/** Año-mes-día (hora LOCAL, mismo criterio que `fechaLocalYMD`) de la fecha
 * `meses` atrás — para valores por defecto de un filtro de rango. `hoy` es
 * inyectable para que el test sea determinístico. Si el mes destino es más
 * corto (31 de agosto − 6 meses cae en un "31 de febrero" inexistente), se
 * ajusta (clamp) al último día válido de ese mes en vez de dejar que rebalse
 * al mes siguiente — mismo criterio que `subMonths` de date-fns. */
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
