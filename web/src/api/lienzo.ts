import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";
import { diasEntre, sumarDias, type Fila } from "../lienzo/calculo";
import { parametrosContratistas, parametrosIngresos } from "../lienzo/consulta";
import { aplicarFiltros } from "../lienzo/filtrar";
import { origenDe, validarLienzo, type Lienzo } from "../lienzo/esquema";
import { invocar } from "./_invocar";

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

export interface TurnoChat {
  rol: "usuario" | "asistente";
  texto: string;
}

export type RespuestaAsistente = { tipo: "editado"; lienzo: Lienzo; mensaje: string } | { tipo: "respondido"; mensaje: string };

const respuestaAsistenteEsquema = z.union([
  z.object({ tipo: z.literal("editado"), lienzo: z.unknown(), mensaje: z.string() }),
  z.object({ tipo: z.literal("respondido"), mensaje: z.string() }),
]);

/**
 * Un pedido al asistente (Edge Function `lienzo-ia`). El documento que
 * devuelve se vuelve a validar acá antes de usarlo: el panel no confía en lo
 * que llega por la red, aunque la función ya lo validó.
 */
export async function preguntarAlAsistente(pedido: {
  pregunta: string;
  lienzo: Lienzo | null;
  historial: TurnoChat[];
}): Promise<RespuestaAsistente> {
  const datos = await invocar(
    "lienzo-ia",
    (d): d is unknown => respuestaAsistenteEsquema.safeParse(d).success,
    { pregunta: pedido.pregunta, lienzo: pedido.lienzo, historial: pedido.historial },
  );
  const respuesta = respuestaAsistenteEsquema.parse(datos);
  if (respuesta.tipo === "respondido") return respuesta;
  const validado = validarLienzo(respuesta.lienzo);
  if (!validado.ok) throw new Error("El asistente devolvió una vista que no se puede dibujar.");
  return { tipo: "editado", lienzo: validado.lienzo, mensaje: respuesta.mensaje };
}

const respuestaConsultaEsquema = z.object({
  columnas: z.array(z.string()),
  filas: z.array(z.record(z.string(), z.unknown())),
  truncado: z.boolean(),
});

/** Un valor de la base como celda del lienzo: lo que no es texto, número o sí/no se muestra como texto. */
function comoCelda(valor: unknown): Fila[string] {
  if (valor === null || valor === undefined) return null;
  if (typeof valor === "string" || typeof valor === "number" || typeof valor === "boolean") return valor;
  return JSON.stringify(valor);
}

/**
 * Corre la consulta del asistente (Edge Function `lienzo-consulta`, sin modelo)
 * y deja las filas como las espera el lienzo: las columnas de fecha con sólo el
 * día, y los filtros del documento aplicados.
 */
async function consultarSql(lienzo: Lienzo): Promise<DatosLienzo> {
  const consulta = lienzo.consulta;
  if (!consulta) throw new Error("El lienzo no trae su consulta.");
  const datos = await invocar(
    "lienzo-consulta",
    (d): d is unknown => respuestaConsultaEsquema.safeParse(d).success,
    { sql: consulta.sql },
  );
  const respuesta = respuestaConsultaEsquema.parse(datos);
  if (respuesta.filas.length > 0) {
    const faltan = consulta.columnas.filter((c) => !respuesta.columnas.includes(c.nombre));
    if (faltan.length > 0) throw new Error(`La consulta no devolvió: ${faltan.map((c) => c.nombre).join(", ")}.`);
  }
  const deFecha = consulta.columnas.filter((c) => c.tipo === "fecha").map((c) => c.nombre);
  const filas = respuesta.filas.map((f) => {
    const fila: Fila = {};
    for (const [nombre, valor] of Object.entries(f)) {
      const celda = comoCelda(valor);
      fila[nombre] = deFecha.includes(nombre) && typeof celda === "string" ? celda.slice(0, 10) : celda;
    }
    return fila;
  });
  return { filas: aplicarFiltros(origenDe(lienzo), filas, lienzo.filtros), truncado: respuesta.truncado };
}

/** Lo que necesita el lienzo para dibujarse. */
export interface DatosLienzo {
  filas: Fila[];
  /** La consulta tenía más filas que el límite: se ven sólo las primeras. */
  truncado?: boolean;
  /** Filas del período anterior del mismo largo (sólo si algún KPI compara). */
  anteriores?: Fila[];
  /** Rango de días consultado (sólo fuentes por día). */
  rango?: { desde: string; hasta: string };
}

/** La función SQL acepta hasta 92 días por consulta. */
const MAXIMO_DIAS = 92;

/** Filas de la fuente del lienzo con sus filtros; `lienzo` ya debe estar validado. */
export async function consultarLienzo(lienzo: Lienzo): Promise<DatosLienzo> {
  if (lienzo.fuente === "consulta") return consultarSql(lienzo);
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

const alertaEsquema = z.object({
  id: z.string(),
  creado_en: z.string(),
  correo: z.string(),
  origen: z.enum(["detector", "modelo", "parser"]),
  motivo: z.string(),
  mensaje: z.string(),
  detalle: z.string().nullable(),
  revisada_en: z.string().nullable(),
});

export type AlertaIa = z.infer<typeof alertaEsquema>;

/** ¿Esta cuenta recibe las alertas del asistente? La base sólo le muestra sus propios alcances. */
export async function puedeVerAlertas(): Promise<boolean> {
  const { data, error } = await supabase.from("ia_permisos").select("alcance").eq("alcance", "alertas").limit(1);
  if (error) throw new Error(error.message);
  return (data ?? []).length > 0;
}

/** Intentos de que el asistente ejecutara SQL o se saltara sus reglas: quién, qué escribió y por qué se marcó. */
export async function listarAlertas(): Promise<AlertaIa[]> {
  const { data, error } = await supabase
    .from("ia_alertas")
    .select("id, creado_en, correo, origen, motivo, mensaje, detalle, revisada_en")
    .order("creado_en", { ascending: false })
    .limit(50);
  if (error) throw new Error(error.message);
  return z.array(alertaEsquema).parse(data);
}

const vistaGuardadaEsquema = z.object({
  id: z.string(),
  nombre: z.string(),
  lienzo: z.unknown(),
  actualizado_en: z.string(),
});

export interface VistaGuardada {
  id: string;
  nombre: string;
  /** Sin validar: se valida al abrirla (un documento viejo puede no pasar el validador de hoy). */
  lienzo: unknown;
  actualizado_en: string;
}

/** Las vistas que guardó esta cuenta (la base no devuelve las de otros). */
export async function listarVistasGuardadas(): Promise<VistaGuardada[]> {
  const { data, error } = await supabase
    .from("lienzo_vistas_guardadas")
    .select("id, nombre, lienzo, actualizado_en")
    .order("actualizado_en", { ascending: false });
  if (error) throw new Error(error.message);
  return z.array(vistaGuardadaEsquema).parse(data);
}

/** Guarda con nombre; si ya hay una vista con ese nombre, la reemplaza. Devuelve su id. */
export async function guardarVista(nombre: string, lienzo: Lienzo): Promise<string> {
  const { data: sesion } = await supabase.auth.getUser();
  const correo = sesion.user?.email;
  if (!correo) throw new Error("No hay una sesión iniciada.");
  const { data, error } = await supabase
    .from("lienzo_vistas_guardadas")
    .upsert({ correo, nombre: nombre.trim(), lienzo, actualizado_en: new Date().toISOString() }, { onConflict: "correo,nombre" })
    .select("id")
    .single();
  if (error) throw new Error(error.message);
  return z.object({ id: z.string() }).parse(data).id;
}

export async function borrarVista(id: string): Promise<void> {
  const { error } = await supabase.from("lienzo_vistas_guardadas").delete().eq("id", id);
  if (error) throw new Error(error.message);
}

export async function marcarAlertaRevisada(id: string): Promise<void> {
  const { error } = await supabase.from("ia_alertas").update({ revisada_en: new Date().toISOString() }).eq("id", id);
  if (error) throw new Error(error.message);
}
