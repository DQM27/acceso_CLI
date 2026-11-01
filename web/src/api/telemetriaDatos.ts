import { supabase } from "../lib/supabase";

/**
 * Los datos de telemetría de producción para el panel (fase 4 de
 * `docs/features-futuras/plan-observabilidad-en-produccion.md`): funciones de
 * la nube que sólo responden al administrador (migración
 * `20261101130000_observabilidad_panel`). Las órdenes de estudios están en
 * `./telemetria.ts`.
 */

/** Último estado de un equipo que manda telemetría. */
export interface EquipoFlota {
  dispositivo_id: string;
  etiqueta: string | null;
  plataforma: "movil" | "escritorio";
  version_app: string;
  version_codigo: number | null;
  sistema: string | null;
  modelo: string | null;
  sesion_iniciada_en: string;
  /** Nulo si todavía no llegó ningún lote. */
  ultimo_lote_en: string | null;
  filas_24h: number;
  descartadas_24h: number;
}

/** Una operación del núcleo en un equipo y una versión. */
export interface RendimientoOperacion {
  dispositivo_id: string;
  etiqueta: string | null;
  plataforma: "movil" | "escritorio";
  version_app: string;
  operacion: string;
  llamadas: number;
  errores: number;
  p50_ms: number | null;
  p90_ms: number | null;
  max_ms: number | null;
  ultima_en: string;
}

/** Una operación en una versión, sumando todos los equipos (resumen diario). */
export interface RendimientoVersion {
  plataforma: "movil" | "escritorio";
  version_app: string;
  primer_dia: string;
  ultimo_dia: string;
  equipos: number;
  llamadas: number;
  errores: number;
  p50_ms: number | null;
  p90_ms: number | null;
  max_ms: number | null;
}

/** Ventanas que ofrece la vista de rendimiento (el detalle se guarda 30 días). */
export const VENTANAS_RENDIMIENTO = [1, 7, 30] as const;
export type VentanaRendimiento = (typeof VENTANAS_RENDIMIENTO)[number];

// PostgREST devuelve `numeric` y `bigint` como número o texto según el valor:
// se normaliza a número para ordenar y formatear.
function numero(valor: unknown): number | null {
  if (valor === null || valor === undefined) return null;
  const n = typeof valor === "number" ? valor : Number(valor);
  return Number.isFinite(n) ? n : null;
}

export async function listarFlotaTelemetria(): Promise<EquipoFlota[]> {
  const { data, error } = await supabase.rpc("panel_telemetria_flota");
  if (error) throw new Error(error.message);
  return ((data ?? []) as EquipoFlota[]).map((fila) => ({
    ...fila,
    filas_24h: numero(fila.filas_24h) ?? 0,
    descartadas_24h: numero(fila.descartadas_24h) ?? 0,
  }));
}

export async function listarRendimiento(dias: VentanaRendimiento): Promise<RendimientoOperacion[]> {
  const { data, error } = await supabase.rpc("panel_telemetria_rendimiento", { p_dias: dias });
  if (error) throw new Error(error.message);
  return ((data ?? []) as RendimientoOperacion[]).map((fila) => ({
    ...fila,
    llamadas: numero(fila.llamadas) ?? 0,
    errores: numero(fila.errores) ?? 0,
    p50_ms: numero(fila.p50_ms),
    p90_ms: numero(fila.p90_ms),
    max_ms: numero(fila.max_ms),
  }));
}

export async function listarRendimientoPorVersion(operacion: string, dias = 365): Promise<RendimientoVersion[]> {
  const { data, error } = await supabase.rpc("panel_telemetria_versiones", { p_operacion: operacion, p_dias: dias });
  if (error) throw new Error(error.message);
  return ((data ?? []) as RendimientoVersion[]).map((fila) => ({
    ...fila,
    equipos: numero(fila.equipos) ?? 0,
    llamadas: numero(fila.llamadas) ?? 0,
    errores: numero(fila.errores) ?? 0,
    p50_ms: numero(fila.p50_ms),
    p90_ms: numero(fila.p90_ms),
    max_ms: numero(fila.max_ms),
  }));
}
