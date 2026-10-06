import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";
import { inicioDiaCostaRicaUtc, inicioDiaSiguienteCostaRicaUtc } from "../tiempo";

/**
 * Resumen agregado de movimientos para la pantalla "Análisis": función
 * `panel_resumen_movimientos` (migración del mismo nombre). La base agrega y
 * el navegador recibe pocos miles de filas, sin importar cuántos millones
 * tenga el historial. RLS: el administrador del panel ve todas las unidades.
 */

export type TipoPersona = "CONTRATISTA" | "PROVEEDOR";

export const TEXTO_TIPO_PERSONA: Record<TipoPersona, string> = {
  CONTRATISTA: "Contratistas",
  PROVEEDOR: "Proveedores",
};

/** Una fila por día × unidad × tipo de persona × tipo de ingreso × medio. */
export interface ResumenDiario {
  /** Año-mes-día de Costa Rica. */
  dia: string;
  unidad: string;
  tipo_persona: TipoPersona;
  tipo_ingreso: string;
  medio: string;
  ingresos: number;
  con_salida: number;
  /** Suma de permanencias de quienes ya salieron. */
  minutos_adentro: number;
}

/** Ingresos por día de la semana (1 = lunes) y hora del día (0-23). */
export interface ResumenPorHora {
  dia_semana: number;
  hora: number;
  ingresos: number;
}

export interface ResumenEmpresa {
  empresa: string;
  tipo_persona: TipoPersona;
  ingresos: number;
  personas: number;
}

export interface ResumenTotal {
  ingresos: number;
  con_salida: number;
  minutos_adentro: number;
  personas: number;
}

export interface ResumenMovimientos {
  diario: ResumenDiario[];
  por_hora: ResumenPorHora[];
  /** Las 100 empresas con más ingresos, de mayor a menor. */
  empresas: ResumenEmpresa[];
  total: ResumenTotal;
}

const tipoPersonaEsquema = z.enum(["CONTRATISTA", "PROVEEDOR"]);

const resumenEsquema = z.object({
  diario: z.array(
    z.object({
      dia: z.string(),
      unidad: z.string(),
      tipo_persona: tipoPersonaEsquema,
      tipo_ingreso: z.string(),
      medio: z.string(),
      ingresos: z.number(),
      con_salida: z.number(),
      minutos_adentro: z.number(),
    }),
  ),
  por_hora: z.array(z.object({ dia_semana: z.number(), hora: z.number(), ingresos: z.number() })),
  empresas: z.array(
    z.object({
      empresa: z.string(),
      tipo_persona: tipoPersonaEsquema,
      ingresos: z.number(),
      personas: z.number(),
    }),
  ),
  total: z.object({
    ingresos: z.number(),
    con_salida: z.number(),
    minutos_adentro: z.number(),
    personas: z.number(),
  }),
});

/** Mismo tope que la función en la base: más de un año de una vez se acerca al
 * corte de 8 s de Supabase con el volumen de todas las unidades. */
export const MAXIMO_DIAS_RESUMEN = 366;

export interface ConsultaResumen {
  /** Año-mes-día (Costa Rica), obligatorio: el resumen siempre es de un período. */
  desde: string;
  /** Año-mes-día (Costa Rica), inclusive; vacío = hasta hoy. */
  hasta?: string;
  /** `undefined` = todas las unidades. */
  sitioIds?: string[];
}

/** Días calendario entre dos año-mes-día, contando los dos extremos. */
export function diasDelRango(desde: string, hasta: string): number {
  const aUtc = (ymd: string) => {
    const [anio, mes, dia] = ymd.split("-").map(Number);
    return Date.UTC(anio, mes - 1, dia);
  };
  return Math.round((aUtc(hasta) - aUtc(desde)) / 86_400_000) + 1;
}

/** Mensaje para la persona si el período no se puede resumir; `null` si sí. */
export function problemaDelRango(desde: string, hasta: string): string | null {
  if (!desde) return "Elija desde qué fecha resumir.";
  const dias = diasDelRango(desde, hasta);
  if (dias < 1) return "La fecha final es anterior a la inicial.";
  if (dias > MAXIMO_DIAS_RESUMEN) return "El análisis admite hasta un año a la vez; acote el período.";
  return null;
}

export async function obtenerResumenMovimientos(
  { desde, hasta, sitioIds }: ConsultaResumen,
  hoy: string,
): Promise<ResumenMovimientos> {
  const { data, error } = await supabase.rpc("panel_resumen_movimientos", {
    p_desde: inicioDiaCostaRicaUtc(desde),
    p_hasta: inicioDiaSiguienteCostaRicaUtc(hasta || hoy),
    p_sitio_ids: sitioIds ?? null,
  });
  if (error) throw new Error(error.message);
  return resumenEsquema.parse(data);
}

/** Permanencia promedio en palabras cortas ("2 h 15 min", "45 min"), o "—"
 * si nadie salió todavía. */
export function textoPermanencia(minutosAdentro: number, conSalida: number): string {
  if (conSalida <= 0) return "—";
  const minutos = Math.round(minutosAdentro / conSalida);
  if (minutos < 60) return `${minutos} min`;
  const horas = Math.floor(minutos / 60);
  return minutos % 60 === 0 ? `${horas} h` : `${horas} h ${minutos % 60} min`;
}
