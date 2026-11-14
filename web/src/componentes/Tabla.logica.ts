import type { ITooltipParams } from "ag-grid-community";

/** Texto del tooltip de una celda cortada ("KAREN DE LOS ANGELE…"): el texto ya
 * formateado como se ve ("S/G", "23/09/2026") si la columna tiene
 * `valueFormatter`, si no el valor crudo; `undefined` (sin tooltip) para
 * cualquier cosa que no sea texto o número. Copiado de
 * `desktop/src/componentes/Tabla.logica.ts`. */
export function textoTooltip({ valueFormatted, value }: ITooltipParams): string | undefined {
  if (typeof valueFormatted === "string" && valueFormatted !== "") return valueFormatted;
  if (typeof value === "string" && value !== "") return value;
  if (typeof value === "number") return String(value);
  return undefined;
}

/** Comparador del filtro de fecha para columnas cuyo valor es "AAAA-MM-DD"
 * (`fechaLocalYMD`, todas las columnas de fecha de la app): el valor se guarda
 * así para que ordene bien como texto; lo que se ve en pantalla es
 * DD/MM/AAAA. */
export function compararFechaYMD(filtroMedianoche: Date, valorCelda: unknown): number {
  if (typeof valorCelda !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(valorCelda)) return -1;
  const [anio, mes, dia] = valorCelda.split("-").map(Number);
  const celda = new Date(anio, mes - 1, dia).getTime();
  const filtro = filtroMedianoche.getTime();
  return celda < filtro ? -1 : celda > filtro ? 1 : 0;
}

/** Identidad de fila (`idFila` de `Tabla`) para las filas con `id`: función
 * de módulo, estable. Una función nueva en cada render (`idFila={(f) => ...}`
 * en línea) hacía que la grilla reconstruyera sus columnas en cada refresco
 * y perdiera los anchos ajustados. */
export const idPorId = (fila: { id: string | number }) => String(fila.id);

/** Sin tildes y en mayúsculas -- mismo criterio de "sin tildes/mayúsculas no
 * importa" que el buscador del núcleo. El `quickFilterText` propio de AG Grid
 * sólo hace `.toUpperCase()`, sin tocar diacríticos: buscar "Sanches" no
 * encontraba a "Sánchez". Igual que `desktop/src/componentes/Tabla.logica.ts`. */
export function plegar(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toUpperCase();
}

/** Tope de textos recordados por `plegarConCache`. Al llenarse se vacía. */
export const TOPE_CACHE_PLEGADO = 20_000;

const cachePlegado = new Map<string, string>();

/** `plegar` recordando el resultado. El buscador compara el texto de CADA fila
 * en cada búsqueda; con `cacheQuickFilter` AG Grid entrega siempre el mismo
 * texto por fila, así que cada uno se pliega una sola vez (auditoría
 * 2026-11-14, R5, igual que en escritorio). */
export function plegarConCache(texto: string): string {
  const guardado = cachePlegado.get(texto);
  if (guardado !== undefined) return guardado;
  if (cachePlegado.size >= TOPE_CACHE_PLEGADO) cachePlegado.clear();
  const plegado = plegar(texto);
  cachePlegado.set(texto, plegado);
  return plegado;
}

/** Para los tests: cuántos textos hay recordados. */
export function tamanoCachePlegado(): number {
  return cachePlegado.size;
}

/** Lo que se lee del layout guardado para decidir el arranque de la grilla. */
interface LayoutGuardado {
  ocultas: string[];
  filtrosVisibles?: boolean;
  filtrosElegidos?: boolean;
}

/** Fila de filtros por columna al abrir la grilla: apagada salvo que la
 * persona la haya prendido a mano (`filtrosElegidos`). Los layouts viejos
 * guardaban `filtrosVisibles` en `true` con cualquier cambio de anchos,
 * aunque nadie la hubiera elegido (auditoría 2026-11-14, U6). */
export function filtrosVisiblesIniciales(guardado: LayoutGuardado | null): boolean {
  return guardado?.filtrosElegidos === true ? guardado.filtrosVisibles === true : false;
}

/** Columnas ocultas al abrir: las que eligió la persona o, si nunca tocó la
 * grilla, las que el código marca `hide: true` (opcionales). */
export function ocultasIniciales(
  guardado: LayoutGuardado | null,
  columnas: { colId?: unknown; field?: unknown; hide?: unknown }[],
): string[] {
  if (guardado) return guardado.ocultas;
  return columnas
    .filter((columna) => columna.hide === true)
    .map((columna) =>
      typeof columna.colId === "string" ? columna.colId : typeof columna.field === "string" ? columna.field : undefined,
    )
    .filter((clave): clave is string => clave !== undefined);
}
