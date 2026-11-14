import type { ColDef, ITooltipParams, ColumnState } from "ag-grid-community";

/** Lo que muestra el tooltip de una celda -- el valor ya formateado (ej.
 * "S/G", "23/09/2026") si la columna tiene `valueFormatter`, si no el valor
 * crudo; `undefined` (sin tooltip) para cualquier cosa que no sea texto o
 * número. */
/** Espejo en JS de `PLEGAR` (SQLite, `database/schema.rs`) -- mismo criterio
 * de "sin tildes/mayúsculas no importa" que ya tiene el buscador de Rust
 * (`database/search.rs`). El `quickFilterText` propio de AG Grid sólo hace
 * `.toUpperCase()`, sin tocar diacríticos -- sin esto, buscar "Sanches" en
 * la grilla no encontraba a "Sánchez" aunque el mismo texto sí funcionara en
 * el buscador del modal "Nuevo ingreso" (hallazgo real del usuario,
 * 2026-09-21: dos buscadores con reglas distintas para el mismo dato). */
export function plegar(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toUpperCase();
}

/** Tope de textos recordados por `plegarConCache`: de sobra para el historial
 * más grande que trae una grilla (ver `LIMITE_CARGA_COMPLETA_MAXIMO`). Al
 * llenarse se vacía entero, sin llevar la cuenta de cuál se usó último. */
export const TOPE_CACHE_PLEGADO = 20_000;

const cachePlegado = new Map<string, string>();

/** `plegar` recordando el resultado. El buscador de las grillas compara el
 * texto de CADA fila en cada tecla; con `cacheQuickFilter` AG Grid entrega
 * siempre el mismo texto por fila, así que cada uno se pliega una sola vez
 * en vez de una vez por tecla (auditoría 2026-11-14, R5). */
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

export function textoTooltip({ valueFormatted, value }: ITooltipParams): string | undefined {
  if (typeof valueFormatted === "string" && valueFormatted !== "") return valueFormatted;
  if (typeof value === "string" && value !== "") return value;
  if (typeof value === "number") return String(value);
  return undefined;
}

/** Un clic dentro de un botón, interruptor o campo de la celda no debe
 * marcar/desmarcar la fila -- ej. el botón "Salida" de Activos ya hace su
 * propia acción. */
export function clicEnControlInteractivo(objetivo: EventTarget | null | undefined): boolean {
  return objetivo instanceof Element && objetivo.closest("button, input, select, a, label") !== null;
}

/** Comparador del filtro de fecha para columnas cuyo valor es "AAAA-MM-DD"
 * (`fechaLocalYMD`, todas las columnas de fecha de la app) -- el valor se
 * guarda así para que ordene bien como texto; lo que se ve en pantalla es
 * DD/MM/AAAA (`valueFormatter`). Un valor que no es fecha (ej. "Activo" en
 * "Fecha salida") cuenta como anterior a cualquier fecha. */
export function compararFechaYMD(filtroMedianoche: Date, valorCelda: unknown): number {
  if (typeof valorCelda !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(valorCelda)) return -1;
  const [anio, mes, dia] = valorCelda.split("-").map(Number);
  const celda = new Date(anio, mes - 1, dia).getTime();
  const filtro = filtroMedianoche.getTime();
  return celda < filtro ? -1 : celda > filtro ? 1 : 0;
}

// v2: el layout guardado incluye `pinned` por columna — al sacar el pin
// fijo de Acción (Activos) del código, un layout viejo lo seguía trayendo
// de vuelta desde acá. Subir la versión descarta ese estado guardado
// obsoleto en vez de tener que migrarlo a mano.
export function claveAlmacenamiento(id: string): string {
  return `tabla:${id}:v2`;
}

export interface EstadoGuardado {
  ocultas: string[];
  columnas: ColumnState[];
  /** `undefined` en layouts guardados antes de que existiera esta opción —
   * se toma como visible (comportamiento de siempre) para no ocultarle a
   * nadie los filtros sin que lo haya pedido. */
  filtrosVisibles?: boolean;
}

export function leerEstadoGuardado(id: string | undefined): EstadoGuardado | null {
  if (!id) return null;
  try {
    const crudo = localStorage.getItem(claveAlmacenamiento(id));
    return crudo ? (JSON.parse(crudo) as EstadoGuardado) : null;
  } catch {
    return null;
  }
}

/** Identidad de una columna para visibilidad/orden — `colId` si está
 * explícito (ej. dos columnas que leen el mismo `field`, como Fecha/Hora),
 * si no el `field`. Mismo criterio que usa AG Grid internamente para su
 * propio `getColumnState`. */
export function identidad(columna: ColDef<unknown>): string | undefined {
  if (typeof columna.colId === "string") return columna.colId;
  if (typeof columna.field === "string") return columna.field;
  return undefined;
}

/** Identidad de fila (`idFila` de `Tabla`) para las filas con `uuid` o con
 * `clave`: funciones de módulo, estables -- una función nueva en cada render
 * hacía que la grilla se reconstruyera y perdiera los anchos ajustados.
 * Estaban repetidas en cada pantalla. */
export const idPorUuid = (fila: { uuid: string }) => fila.uuid;
export const idPorClave = (fila: { clave: string }) => fila.clave;
