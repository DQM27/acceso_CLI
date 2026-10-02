import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";

/**
 * Quién está adentro ahora, por unidad: contratistas, proveedores y
 * préstamos de gafete provisional KOF sin salida. Vista
 * `panel_adentro_ahora` (migración `vistas_estado_y_adentro`): sólo lee
 * filas abiertas con índices parciales, así que no depende del tamaño del
 * historial. RLS: el administrador del panel ve todas las unidades.
 */
export type TipoAdentro = "CONTRATISTA" | "PROVEEDOR" | "PROVISIONAL_KOF";

export interface PersonaAdentro {
  tipo: TipoAdentro;
  id: string;
  sitio_id: string;
  sitio_nombre: string | null;
  /** Cédula; en KOF, el código de empleado del encargado. */
  identificacion: string | null;
  nombre: string;
  empresa_nombre: string | null;
  gafete_numero: number | null;
  placa: string | null;
  hora_entrada: string;
  usuario_entrada_nombre: string | null;
}

export const TEXTO_TIPO_ADENTRO: Record<TipoAdentro, string> = {
  CONTRATISTA: "Contratista",
  PROVEEDOR: "Proveedor",
  PROVISIONAL_KOF: "Provisional KOF",
};

const personaAdentroEsquema = z.object({
  tipo: z.enum(["CONTRATISTA", "PROVEEDOR", "PROVISIONAL_KOF"]),
  id: z.string(),
  sitio_id: z.string(),
  sitio_nombre: z.string().nullable(),
  identificacion: z.string().nullable(),
  nombre: z.string(),
  empresa_nombre: z.string().nullable(),
  gafete_numero: z.number().nullable(),
  placa: z.string().nullable(),
  hora_entrada: z.string(),
  usuario_entrada_nombre: z.string().nullable(),
});

// Válvula de seguridad, no paginación: nadie tiene miles de personas
// adentro a la vez; si pasa, es señal de salidas sin marcar.
const LIMITE_ADENTRO = 5_000;

export async function listarAdentroAhora(): Promise<PersonaAdentro[]> {
  const { data, error } = await supabase
    .from("panel_adentro_ahora")
    .select(
      "tipo, id, sitio_id, sitio_nombre, identificacion, nombre, empresa_nombre, " +
        "gafete_numero, placa, hora_entrada, usuario_entrada_nombre",
    )
    .order("hora_entrada", { ascending: true })
    .range(0, LIMITE_ADENTRO - 1);
  if (error) throw new Error(error.message);
  return z.array(personaAdentroEsquema).parse(data);
}

/** Tiempo adentro en palabras cortas: "45 min", "3 h 20 min", "2 d 4 h". */
export function textoTiempoAdentro(desdeIso: string, ahora: Date = new Date()): string {
  const minutos = Math.max(0, Math.floor((ahora.getTime() - new Date(desdeIso).getTime()) / 60_000));
  if (minutos < 60) return `${minutos} min`;
  const horas = Math.floor(minutos / 60);
  if (horas < 24) return minutos % 60 === 0 ? `${horas} h` : `${horas} h ${minutos % 60} min`;
  const diasCompletos = Math.floor(horas / 24);
  return horas % 24 === 0 ? `${diasCompletos} d` : `${diasCompletos} d ${horas % 24} h`;
}

/** Horas adentro a partir de las cuales la fila se resalta: casi siempre
 * es una salida que no se marcó. */
export const HORAS_ALERTA_ADENTRO = 12;

export function llevaDemasiado(desdeIso: string, ahora: Date = new Date()): boolean {
  return ahora.getTime() - new Date(desdeIso).getTime() >= HORAS_ALERTA_ADENTRO * 3_600_000;
}
