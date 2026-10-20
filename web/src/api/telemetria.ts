import { supabase } from "../lib/supabase";

/**
 * Estudios de telemetría por equipo (fase 3 del plan de telemetría, ver
 * `docs/handoff-telemetria-por-estudios.md`). El panel escribe
 * `telemetria_ordenes` (RLS: sólo el administrador del panel) y un trigger de
 * la nube sella versión, autor y hora, guarda el historial y avisa al equipo
 * por Realtime. El panel no toca las apps.
 *
 * `base` no se guarda: va siempre. Un equipo sin fila mide `base` y
 * `rendimiento`; al vencer la orden vuelve a eso solo, aunque no tenga red.
 */

/** Lista fija, igual que el `check` de la tabla: agregar un grupo es una
 * migración y un cambio en las apps. */
export const GRUPOS_TELEMETRIA = [
  "rendimiento",
  "fluidez",
  "sincronizacion",
  "recursos",
  "escaneo",
  "depuracion",
] as const;

export type GrupoTelemetria = (typeof GRUPOS_TELEMETRIA)[number];

/** Lo que mide un equipo sin orden (y al vencer la suya), además de `base`. */
export const GRUPOS_POR_DEFECTO: readonly GrupoTelemetria[] = ["rendimiento"];

/** Mismo tope que el `check` de la columna `nota`. */
export const LARGO_MAXIMO_NOTA = 200;

export interface OrdenTelemetria {
  dispositivo_id: string;
  grupos: GrupoTelemetria[];
  /** Nulo = sin vencimiento. */
  vence_en: string | null;
  version: number;
  nota: string;
  actualizado_en: string;
  actualizado_por: string;
}

export interface CambioOrdenTelemetria {
  id: number;
  dispositivo_id: string;
  /** Guardada al momento del cambio: el historial sobrevive al retiro del equipo. */
  etiqueta: string | null;
  grupos: GrupoTelemetria[];
  vence_en: string | null;
  version: number;
  nota: string;
  cambiado_por: string;
  cambiado_en: string;
}

/** Cuántos cambios muestra el historial: los últimos, no hace falta más. */
export const CAMBIOS_EN_HISTORIAL = 50;

/** Las órdenes de todos los equipos que tienen una, por `dispositivo_id`. */
export async function listarOrdenesTelemetria(): Promise<Map<string, OrdenTelemetria>> {
  const { data, error } = await supabase
    .from("telemetria_ordenes")
    .select("dispositivo_id, grupos, vence_en, version, nota, actualizado_en, actualizado_por");
  if (error) throw new Error(error.message);
  return new Map(((data ?? []) as OrdenTelemetria[]).map((orden) => [orden.dispositivo_id, orden]));
}

/** Los últimos cambios de órdenes, del más nuevo al más viejo. */
export async function listarCambiosTelemetria(): Promise<CambioOrdenTelemetria[]> {
  const { data, error } = await supabase
    .from("telemetria_ordenes_historial")
    .select("id, dispositivo_id, etiqueta, grupos, vence_en, version, nota, cambiado_por, cambiado_en")
    .order("cambiado_en", { ascending: false })
    .limit(CAMBIOS_EN_HISTORIAL);
  if (error) throw new Error(error.message);
  return (data ?? []) as CambioOrdenTelemetria[];
}

/**
 * Crea o reemplaza la orden de un equipo. Sin DELETE a propósito: volver a lo
 * de siempre es guardar `GRUPOS_POR_DEFECTO` sin vencimiento. Versión, autor y
 * hora los pone el trigger, no el panel.
 */
export async function guardarOrdenTelemetria(orden: {
  dispositivo_id: string;
  grupos: readonly GrupoTelemetria[];
  vence_en: string | null;
  nota: string;
}): Promise<void> {
  const { error } = await supabase
    .from("telemetria_ordenes")
    .upsert({ ...orden, grupos: [...orden.grupos] }, { onConflict: "dispositivo_id" });
  if (error) throw new Error(error.message);
}
