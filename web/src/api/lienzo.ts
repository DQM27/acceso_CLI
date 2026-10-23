import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";
import { diasEntre, sumarDias, type Fila } from "../lienzo/calculo";
import { parametrosContratistas, parametrosIngresos } from "../lienzo/consulta";
import type { Lienzo } from "../lienzo/esquema";

/**
 * Consultas del lienzo de datos (Asistente): llaman a las funciones de sólo
 * lectura `lienzo_*` (migración `lienzo_catalogo`). Sólo las usa un administrador
 * del panel: la propia función lo comprueba y rechaza al resto.
 */
const resumenEsquema = z.object({
  hoy: z.string(),
  adentro_contratistas: z.number(),
  adentro_proveedores: z.number(),
  ingresos_hoy: z.number(),
  praind_vencidos: z.number(),
  praind_por_vencer: z.number(),
  ingresos_por_dia: z.array(z.object({ dia: z.string(), cantidad: z.number() })),
});

export type ResumenHoy = z.infer<typeof resumenEsquema>;

export async function obtenerResumenHoy(sitioId?: string): Promise<ResumenHoy> {
  const { data, error } = await supabase.rpc("lienzo_resumen_hoy", { p_sitio_id: sitioId ?? undefined });
  if (error) throw new Error(error.message);
  return resumenEsquema.parse(data);
}

const filasEsquema = z.array(z.record(z.string(), z.union([z.string(), z.number(), z.boolean(), z.null()])));

/** Día de hoy en Costa Rica (YYYY-MM-DD), el mismo que usan las funciones SQL. */
export function hoyCostaRica(ahora: Date = new Date()): string {
  return new Intl.DateTimeFormat("en-CA", { timeZone: "America/Costa_Rica" }).format(ahora);
}

/** Lo que necesita el lienzo para dibujarse. */
export interface DatosLienzo {
  filas: Fila[];
  /** Filas del período anterior del mismo largo (sólo si algún KPI compara). */
  anteriores?: Fila[];
  /** Rango de días consultado (sólo fuentes por día). */
  rango?: { desde: string; hasta: string };
}

/** La función SQL acepta hasta 92 días por consulta. */
const MAXIMO_DIAS = 92;

/** Filas de la fuente del lienzo con sus filtros; `lienzo` ya debe estar validado. */
export async function consultarLienzo(lienzo: Lienzo): Promise<DatosLienzo> {
  if (lienzo.fuente === "contratistas") {
    const { data, error } = await supabase.rpc("lienzo_contratistas", parametrosContratistas(lienzo));
    if (error) throw new Error(error.message);
    return { filas: filasEsquema.parse(data) };
  }

  const parametros = parametrosIngresos(lienzo, hoyCostaRica());
  const rango = { desde: parametros.p_desde, hasta: parametros.p_hasta };
  // Si algún KPI compara, se pide de una vez también el período anterior del mismo largo.
  const largo = diasEntre(rango.desde, rango.hasta) + 1;
  const compara = lienzo.widgets.some((w) => w.tipo === "kpi" && w.comparar) && largo * 2 - 1 <= MAXIMO_DIAS;
  const desdePedido = compara ? sumarDias(rango.desde, -largo) : rango.desde;
  const { data, error } = await supabase.rpc("lienzo_ingresos", { ...parametros, p_desde: desdePedido });
  if (error) throw new Error(error.message);
  const todas = filasEsquema.parse(data);
  if (!compara) return { filas: todas, rango };
  const actual = (f: Fila) => typeof f.dia === "string" && f.dia >= rango.desde;
  return { filas: todas.filter(actual), anteriores: todas.filter((f) => !actual(f)), rango };
}
