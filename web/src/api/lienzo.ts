import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";
import type { Fila } from "../lienzo/calculo";
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

/** Filas de la fuente del lienzo con sus filtros; `lienzo` ya debe estar validado. */
export async function consultarFuente(lienzo: Lienzo): Promise<Fila[]> {
  const { data, error } =
    lienzo.fuente === "contratistas"
      ? await supabase.rpc("lienzo_contratistas", parametrosContratistas(lienzo))
      : await supabase.rpc("lienzo_ingresos", parametrosIngresos(lienzo, hoyCostaRica()));
  if (error) throw new Error(error.message);
  return filasEsquema.parse(data);
}
