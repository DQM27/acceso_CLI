import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";
import { inicioDiaCostaRicaUtc, inicioDiaSiguienteCostaRicaUtc } from "../tiempo";

/**
 * Bitácora de sesiones: cada inicio de sesión en escritorio o celular, con
 * usuario, unidad, equipo, fecha y hora de inicio y de cierre, y el motivo
 * del cierre. Vista `panel_bitacora_sesiones` (migración
 * `sesion_unica_por_unidad`); la lee sólo el administrador del panel.
 */
export type MotivoCierre = "salida" | "otra_unidad" | "desplazada" | "sin_cierre";

export interface SesionBitacora {
  id: number;
  cedula: string;
  nombre: string;
  dispositivo_etiqueta: string | null;
  dispositivo_tipo: string | null;
  sitio_id: string | null;
  sitio_nombre: string | null;
  iniciada_en: string;
  cerrada_en: string | null;
  motivo_cierre: MotivoCierre | null;
  /** Cierre, o la última sincronización del equipo si sigue abierta. */
  ultima_actividad: string;
  abierta: boolean;
}

export const TEXTO_MOTIVO_CIERRE: Record<MotivoCierre, string> = {
  salida: "Salió",
  otra_unidad: "Entró en otra unidad",
  desplazada: "Desplazada por otra unidad",
  sin_cierre: "Sin cierre registrado",
};

/** Estado de la sesión en palabras: "Abierta" o el motivo del cierre. */
export function textoEstadoSesion(sesion: Pick<SesionBitacora, "abierta" | "motivo_cierre">): string {
  if (sesion.abierta || !sesion.motivo_cierre) return "Abierta";
  return TEXTO_MOTIVO_CIERRE[sesion.motivo_cierre];
}

/** Duración en palabras cortas ("45 min", "3 h 20 min", "2 d 4 h") entre el
 * inicio y el cierre, o hasta la última actividad si sigue abierta. */
export function textoDuracion(desdeIso: string, hastaIso: string): string {
  const minutos = Math.max(0, Math.floor((Date.parse(hastaIso) - Date.parse(desdeIso)) / 60_000));
  if (minutos < 60) return `${minutos} min`;
  const horas = Math.floor(minutos / 60);
  if (horas < 24) return minutos % 60 === 0 ? `${horas} h` : `${horas} h ${minutos % 60} min`;
  const dias = Math.floor(horas / 24);
  return horas % 24 === 0 ? `${dias} d` : `${dias} d ${horas % 24} h`;
}

const sesionEsquema = z.object({
  id: z.number(),
  cedula: z.string(),
  nombre: z.string(),
  dispositivo_etiqueta: z.string().nullable(),
  dispositivo_tipo: z.string().nullable(),
  sitio_id: z.string().nullable(),
  sitio_nombre: z.string().nullable(),
  iniciada_en: z.string(),
  cerrada_en: z.string().nullable(),
  motivo_cierre: z.enum(["salida", "otra_unidad", "desplazada", "sin_cierre"]).nullable(),
  ultima_actividad: z.string(),
  abierta: z.boolean(),
});

const COLUMNAS =
  "id, cedula, nombre, dispositivo_etiqueta, dispositivo_tipo, sitio_id, sitio_nombre, " +
  "iniciada_en, cerrada_en, motivo_cierre, ultima_actividad, abierta";

/** Tope de filas de una consulta: válvula de seguridad, no paginación. Son
 * unas decenas de sesiones por día y unidad; si se llega, se avisa que hay
 * que acotar las fechas. */
export const MAXIMO_SESIONES = 5_000;

export interface ResultadoSesiones {
  filas: SesionBitacora[];
  /** `true` si el rango tiene más de `MAXIMO_SESIONES` sesiones. */
  truncado: boolean;
}

/** Sesiones iniciadas en el rango (días calendario de Costa Rica, extremos
 * incluidos), de la más reciente a la más vieja. Pide una fila de más para
 * saber si el rango quedó truncado sin contar todo. */
export async function listarSesiones({ desde, hasta }: { desde?: string; hasta?: string }): Promise<ResultadoSesiones> {
  let consulta = supabase
    .from("panel_bitacora_sesiones")
    .select(COLUMNAS)
    .order("iniciada_en", { ascending: false })
    .order("id", { ascending: false })
    .range(0, MAXIMO_SESIONES);
  if (desde) consulta = consulta.gte("iniciada_en", inicioDiaCostaRicaUtc(desde));
  if (hasta) consulta = consulta.lt("iniciada_en", inicioDiaSiguienteCostaRicaUtc(hasta));

  const { data, error } = await consulta;
  if (error) throw new Error(error.message);
  const filas = z.array(sesionEsquema).parse(data);
  return { filas: filas.slice(0, MAXIMO_SESIONES), truncado: filas.length > MAXIMO_SESIONES };
}
