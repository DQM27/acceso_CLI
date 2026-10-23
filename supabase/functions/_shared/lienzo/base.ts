import type { SupabaseClient } from "jsr:@supabase/supabase-js@2";
import type { Alerta } from "./alertas.ts";

/**
 * Lo que `lienzo-ia` y `lienzo-consulta` hacen contra la base con el cliente
 * de servicio: ejecutar una consulta ya revisada (`ia_consultar`), describir
 * las vistas de la cuenta (`ia_describir`) y guardar alertas (`ia_alertas`).
 */

export type ResultadoConsulta =
  | { ok: true; columnas: string[]; filas: Record<string, unknown>[]; truncado: boolean }
  | { ok: false; error: string };

/** Ejecuta `sql` (revisado antes con `revisarSql`) como la cuenta: solo lectura, rol de lectura de `ia`. */
export async function ejecutarConsulta(
  supabase: SupabaseClient,
  correo: string,
  sql: string,
  limite: number,
): Promise<ResultadoConsulta> {
  const { data, error } = await supabase.rpc("ia_consultar", { p_sql: sql, p_correo: correo, p_limite: limite });
  if (error) return { ok: false, error: error.message };
  return { ok: true, columnas: data.columnas, filas: data.filas, truncado: data.truncado };
}

export async function describirVistas(
  supabase: SupabaseClient,
  correo: string,
): Promise<{ vista: string; descripcion: string | null; columnas: { nombre: string; tipo: string }[] }[]> {
  const { data, error } = await supabase.rpc("ia_describir", { p_correo: correo });
  if (error) throw new Error(error.message);
  return data;
}

/** Guarda la alerta; si no se puede, lo deja en el log de la función (nunca tumba el pedido). */
export async function guardarAlerta(supabase: SupabaseClient, alerta: Alerta): Promise<void> {
  const { error } = await supabase.from("ia_alertas").insert({
    correo: alerta.correo,
    origen: alerta.origen,
    motivo: alerta.motivo.slice(0, 500),
    mensaje: alerta.mensaje.slice(0, 4000) || "(vacío)",
    detalle: alerta.detalle?.slice(0, 8000) ?? null,
  });
  if (error) console.error("lienzo: no se pudo guardar la alerta:", error.message, JSON.stringify(alerta).slice(0, 500));
}
