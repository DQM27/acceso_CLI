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

/**
 * "con_orden": el equipo mide algo distinto de lo de siempre (más estudios, o
 * menos). "siempre": sin orden, con la orden vencida o con una orden que pide
 * justo lo de siempre.
 */
export type EstadoEstudio = "con_orden" | "siempre";

export function estadoEstudio(orden: OrdenVigente | undefined, ahora: Date): EstadoEstudio {
  return esLoDeSiempre(gruposVigentes(orden, ahora)) ? "siempre" : "con_orden";
}

/** Cuántos equipos hay en cada estado, para el filtro del panel. */
export function contarPorEstado(estados: readonly EstadoEstudio[]): Record<EstadoEstudio, number> {
  const conteo: Record<EstadoEstudio, number> = { con_orden: 0, siempre: 0 };
  for (const estado of estados) conteo[estado] += 1;
  return conteo;
}

/**
 * Hasta cuándo rige la orden, para la columna "Vence": "—" si no hay orden
 * vigente, "Sin vencimiento", o la fecha con lo que falta ("en 5 h", "en 3 d").
 * `fecha` da el texto de la fecha (la hora del navegador la pone la pantalla).
 */
export function textoVence(
  orden: OrdenVigente | undefined,
  ahora: Date,
  fecha: (iso: string) => string,
): string {
  if (!orden || ordenVencida(orden, ahora)) return "—";
  if (orden.vence_en === null) return "Sin vencimiento";
  const horas = (new Date(orden.vence_en).getTime() - ahora.getTime()) / 3_600_000;
  const falta = horas < 1 ? "menos de 1 h" : horas < 48 ? `${Math.floor(horas)} h` : `${Math.floor(horas / 24)} d`;
  return `${fecha(orden.vence_en)} (en ${falta})`;
}

/** Hora de vencimiento de una orden guardada ahora; `null` = sin vencimiento. */
export function venceEnDesde(horas: number | null, ahora: Date): string | null {
  if (horas === null) return null;
  return new Date(ahora.getTime() + horas * 3_600_000).toISOString();
}

/** Grupos en el orden fijo de la lista, sin repetidos (el panel los marca en
 * cualquier orden). */
export function ordenarGrupos(
  marcados: ReadonlySet<GrupoTelemetria>,
  todos: readonly GrupoTelemetria[],
): GrupoTelemetria[] {
  return todos.filter((grupo) => marcados.has(grupo));
}
