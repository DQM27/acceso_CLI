import { plegarTexto } from "../api/historialFiltros";
import { GRUPOS_POR_DEFECTO } from "../api/telemetria";
import type { GrupoTelemetria } from "../api/telemetria";

/** Qué estudia cada grupo, para la casilla del panel. */
export const DESCRIPCION_GRUPO: Record<GrupoTelemetria, string> = {
  rendimiento: "Rendimiento: cuánto espera el operador (llamadas, ingresos, pantallas)",
  fluidez: "Fluidez: trabas de la interfaz",
  sincronizacion: "Sincronización: llegada de datos, latencia entre equipos y reloj",
  recursos: "Recursos: memoria, CPU y batería (el que más datos manda)",
  escaneo: "Escaneo: calibrar OCR y cámara",
  depuracion: "Depuración: cazar fugas (cuesta rendimiento: sólo por poco tiempo)",
};

/** Vencimientos que ofrece el panel; `null` = sin vencimiento. */
export const OPCIONES_VENCIMIENTO: readonly { etiqueta: string; horas: number | null }[] = [
  { etiqueta: "24 horas", horas: 24 },
  { etiqueta: "3 días", horas: 72 },
  { etiqueta: "7 días", horas: 168 },
  { etiqueta: "30 días", horas: 720 },
  { etiqueta: "Sin vencimiento", horas: null },
];

/** Lo mínimo de una orden para saber qué mide hoy el equipo. */
export interface OrdenVigente {
  grupos: readonly GrupoTelemetria[];
  vence_en: string | null;
}

/** La orden ya no rige: el equipo volvió solo a lo de siempre. */
export function ordenVencida(orden: OrdenVigente, ahora: Date): boolean {
  return orden.vence_en !== null && new Date(orden.vence_en).getTime() <= ahora.getTime();
}

/**
 * Lo que mide el equipo ahora, además de `base`: lo de su orden, o lo de
 * siempre si no tiene o ya venció (el equipo hace cumplir el vencimiento solo).
 */
export function gruposVigentes(orden: OrdenVigente | undefined, ahora: Date): readonly GrupoTelemetria[] {
  if (!orden || ordenVencida(orden, ahora)) return GRUPOS_POR_DEFECTO;
  return orden.grupos;
}

/** "base + rendimiento, fluidez" / "Sólo base". */
export function textoGrupos(grupos: readonly GrupoTelemetria[]): string {
  if (grupos.length === 0) return "Sólo base";
  return `base + ${grupos.join(", ")}`;
}

/** Es exactamente lo que el equipo mide sin orden (el orden no importa). */
export function esLoDeSiempre(grupos: readonly GrupoTelemetria[]): boolean {
  return grupos.length === GRUPOS_POR_DEFECTO.length && GRUPOS_POR_DEFECTO.every((g) => grupos.includes(g));
}

/** Hora de vencimiento de una orden guardada ahora; `null` = sin vencimiento. */
export function venceEnDesde(horas: number | null, ahora: Date): string | null {
  if (horas === null) return null;
  return new Date(ahora.getTime() + horas * 3_600_000).toISOString();
}

/**
 * El buscador de equipos: cada palabra tecleada tiene que aparecer en alguno
 * de los textos (etiqueta, unidad, tipo), sin importar tildes ni mayúsculas.
 * Sin nada tecleado coincide todo.
 */
export function coincideBusqueda(textos: readonly string[], busqueda: string): boolean {
  const palabras = plegarTexto(busqueda).split(/\s+/).filter(Boolean);
  const donde = plegarTexto(textos.join(" "));
  return palabras.every((palabra) => donde.includes(palabra));
}

/** Grupos en el orden fijo de la lista, sin repetidos (el panel los marca en
 * cualquier orden). */
export function ordenarGrupos(
  marcados: ReadonlySet<GrupoTelemetria>,
  todos: readonly GrupoTelemetria[],
): GrupoTelemetria[] {
  return todos.filter((grupo) => marcados.has(grupo));
}
