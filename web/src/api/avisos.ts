import { supabase } from "../lib/supabase";
import { esObjeto, invocar } from "./_invocar";

/**
 * Notificaciones push a los teléfonos (llegan aunque la app esté cerrada):
 * Edge Function `admin-enviar-push` (ver supabase/functions/_shared/fcm.ts).
 * El destino lo elige el administrador: todos, una o varias unidades y/o
 * equipos concretos.
 */

export type TipoAviso = "normal" | "emergente";

/** Mismos topes que valida la Edge Function. */
export const LARGO_MAXIMO_TITULO = 100;
export const LARGO_MAXIMO_CUERPO = 500;

export type DestinoAviso = { todos: true } | { sitio_ids: string[]; dispositivo_ids: string[] };

export interface ResultadoEnvioAviso {
  /** Fila del aviso en el historial (`avisos_enviados`). */
  id: string;
  destinatarios: number;
  enviados: number;
  fallidos: number;
  tokens_eliminados: number;
  /** Etiquetas de los equipos a los que no llegó. */
  no_recibieron: string[];
}

function esResultadoEnvio(valor: unknown): valor is ResultadoEnvioAviso {
  return (
    esObjeto(valor) &&
    typeof valor.id === "string" &&
    typeof valor.destinatarios === "number" &&
    typeof valor.enviados === "number" &&
    typeof valor.fallidos === "number" &&
    typeof valor.tokens_eliminados === "number" &&
    Array.isArray(valor.no_recibieron)
  );
}

/** Una fila del historial (`avisos_enviados`): la guarda la Edge Function en
 * cada envío. */
export interface AvisoEnviado {
  id: string;
  titulo: string;
  cuerpo: string;
  tipo: TipoAviso;
  destino: DestinoAviso;
  enviado_por: string;
  enviado_en: string;
  destinatarios: number;
  enviados: number;
  no_recibieron: string[];
}

/** Cuántos avisos muestra el historial: los últimos, no hace falta más. */
export const AVISOS_EN_HISTORIAL = 50;

/** Los últimos avisos enviados, del más nuevo al más viejo. Sólo los lee el
 * administrador del panel (RLS "admin_global lee los avisos enviados"). */
export async function listarAvisosEnviados(): Promise<AvisoEnviado[]> {
  const { data, error } = await supabase
    .from("avisos_enviados")
    .select("id, titulo, cuerpo, tipo, destino, enviado_por, enviado_en, destinatarios, enviados, no_recibieron")
    .order("enviado_en", { ascending: false })
    .limit(AVISOS_EN_HISTORIAL);
  if (error) throw new Error(error.message);
  return (data ?? []) as AvisoEnviado[];
}

export function enviarAviso(datos: {
  titulo: string;
  cuerpo: string;
  tipo: TipoAviso;
  destino: DestinoAviso;
}): Promise<ResultadoEnvioAviso> {
  return invocar("admin-enviar-push", esResultadoEnvio, datos);
}

/**
 * Equipos que ya registraron su token de notificaciones (la app lo hace al
 * iniciar sesión). La tabla sólo la lee el administrador del panel (RLS
 * "admin_global lee los tokens push"); el token en sí no hace falta acá.
 */
export async function listarEquiposConNotificaciones(): Promise<Set<string>> {
  const { data, error } = await supabase.from("tokens_push").select("dispositivo_id");
  if (error) throw new Error(error.message);
  return new Set((data ?? []).map((fila) => String(fila.dispositivo_id)));
}
