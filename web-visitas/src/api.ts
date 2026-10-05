import { z } from "./lib/validacion";
import { supabase } from "./lib/supabase";
import {
  citaEsquema,
  CitaInvalida,
  validarCita,
  llegadaEsquema,
  sitioEsquema,
  visitanteAnteriorEsquema,
  MAX_SITIOS,
} from "./dominio";
import type { FormularioCita } from "./dominio";
import { hoyCostaRica } from "./fecha";

export const TAMANO_HISTORIAL = 20;
export const CAMPOS_CITA =
  "id,anfitrion_correo,motivo,fecha_desde,fecha_hasta,hora_estimada,estado,created_at,cita_visitantes(id,nombre,cedula,empresa,placa_vehiculo),cita_sitios(sitio_id,sitios(id,nombre))";

/** Mensaje para la persona. Los errores de las funciones de la base ya
 * vienen en español y se muestran tal cual; los técnicos se traducen. */
export function mensajeError(error: unknown): string {
  const objeto = typeof error === "object" && error !== null ? (error as Record<string, unknown>) : {};
  const codigo = typeof objeto.code === "string" ? objeto.code : undefined;
  const mensaje = typeof objeto.message === "string" ? objeto.message : undefined;
  if (codigo === "PGRST202" || codigo === "42P17" || codigo === "42883")
    return "El servicio de visitas no está disponible en este momento. Intente más tarde o avise a administración.";
  if (codigo === "42501" || codigo === "PGRST301")
    return "Su cuenta no tiene permiso para esta acción. Vuelva a iniciar sesión.";
  if (codigo === "23505") return "Esta solicitud ya existe. Actualice la lista antes de intentarlo otra vez.";
  if (codigo === "54000" && mensaje) return mensaje;
  // `raise exception` de las funciones (P0001/P0002): texto ya pensado para la persona.
  if ((codigo === "P0001" || codigo === "P0002") && mensaje) return mensaje;
  return "No se pudo completar la solicitud. Revise su conexión e intente de nuevo.";
}

export async function listarSitios(signal?: AbortSignal) {
  let consulta = supabase.from("sitios").select("id,nombre").order("nombre").order("id").limit(MAX_SITIOS + 1);
  if (signal) consulta = consulta.abortSignal(signal);
  const { data, error } = await consulta;
  if (error) throw error;
  const sitios = z.array(sitioEsquema).parse(data);
  if (sitios.length > MAX_SITIOS) throw new Error("El catálogo de lugares excede el límite de esta versión.");
  return sitios;
}

/** Citas vigentes de hoy en adelante (para "Hoy" y "Próximas"). */
export async function listarCitasActuales(correo: string, signal?: AbortSignal) {
  z.email().parse(correo);
  let consulta = supabase
    .from("citas")
    .select(CAMPOS_CITA)
    .eq("anfitrion_correo", correo)
    .eq("estado", "VIGENTE")
    .gte("fecha_hasta", hoyCostaRica())
    .order("fecha_desde")
    .limit(200);
  if (signal) consulta = consulta.abortSignal(signal);
  const { data, error } = await consulta;
  if (error) throw error;
  return z.array(citaEsquema).parse(data);
}

/** Historial: canceladas y vencidas, de la más reciente a la más vieja. */
export async function listarHistorial(correo: string, pagina: number, signal?: AbortSignal) {
  z.email().parse(correo);
  z.number().int().nonnegative().max(100_000).parse(pagina);
  let consulta = supabase
    .from("citas")
    .select(CAMPOS_CITA)
    .eq("anfitrion_correo", correo)
    .or(`estado.eq.CANCELADA,fecha_hasta.lt.${hoyCostaRica()}`)
    .order("fecha_desde", { ascending: false })
    .order("id", { ascending: false })
    .range(pagina * TAMANO_HISTORIAL, (pagina + 1) * TAMANO_HISTORIAL);
  if (signal) consulta = consulta.abortSignal(signal);
  const { data, error } = await consulta;
  if (error) throw error;
  const citas = z.array(citaEsquema).parse(data);
  return { citas: citas.slice(0, TAMANO_HISTORIAL), hayMas: citas.length > TAMANO_HISTORIAL };
}

export async function obtenerCita(id: string, signal?: AbortSignal) {
  z.uuid().parse(id);
  let consulta = supabase.from("citas").select(CAMPOS_CITA).eq("id", id);
  if (signal) consulta = consulta.abortSignal(signal);
  const { data, error } = await consulta.maybeSingle();
  if (error) throw error;
  return data ? citaEsquema.parse(data) : null;
}

/** Quién llegó y a qué hora, para esas citas (sólo las del anfitrión). */
export async function llegadasDe(citas: string[]) {
  if (citas.length === 0) return [];
  const { data, error } = await supabase.rpc("estado_visitantes_de_mis_citas", { p_citas: citas.slice(0, 200) });
  if (error) throw error;
  return z.array(llegadaEsquema).parse(data);
}

/** Personas que el anfitrión ya agendó, para no volver a escribirlas. */
export async function visitantesAnteriores(busqueda: string, signal?: AbortSignal) {
  let consulta = supabase.rpc("visitantes_anteriores", { p_busqueda: busqueda.trim() || null });
  if (signal) consulta = consulta.abortSignal(signal);
  const { data, error } = await consulta;
  if (error) throw error;
  return z.array(visitanteAnteriorEsquema).parse(data);
}

function parametros(formulario: FormularioCita, fechaValidacion: string) {
  // Las reglas del núcleo (WebAssembly): la base vuelve a revisar todo.
  const resultado = validarCita(formulario, fechaValidacion);
  if (!resultado.ok) throw new CitaInvalida(resultado.errores);
  const datos = resultado.datos;
  return {
    p_fecha_desde: datos.fecha_desde,
    p_fecha_hasta: datos.fecha_hasta,
    p_motivo: datos.motivo,
    p_sitios: datos.sitios,
    p_visitantes: datos.visitantes,
    p_hora_estimada: datos.hora_estimada,
  };
}

/** Agenda una cita. `id` lo genera el cliente: un reintento con el mismo id y
 * el mismo contenido devuelve la misma cita (no duplica). */
export async function crearCita(id: string, formulario: FormularioCita, fechaValidacion = hoyCostaRica()) {
  z.uuid().parse(id);
  const { data, error } = await supabase.rpc("crear_cita_anfitrion", { p_id: id, ...parametros(formulario, fechaValidacion) });
  if (error) throw error;
  const confirmado = z.uuid().parse(data);
  if (confirmado !== id) throw new Error("La respuesta no corresponde a esta solicitud.");
  return confirmado;
}

/** Edita una cita: la base cancela la vieja y crea la nueva (`nuevoId`) en un
 * solo paso, sólo si nadie entró todavía. Así el cambio llega a todas las
 * porterías. Devuelve el id de la cita nueva. */
export async function editarCita(id: string, nuevoId: string, formulario: FormularioCita, fechaValidacion = hoyCostaRica()) {
  z.uuid().parse(id);
  z.uuid().parse(nuevoId);
  const { data, error } = await supabase.rpc("editar_cita_anfitrion", {
    p_id: id,
    p_nuevo_id: nuevoId,
    ...parametros(formulario, fechaValidacion),
  });
  if (error) throw error;
  const confirmado = z.uuid().parse(data);
  if (confirmado !== nuevoId) throw new Error("La respuesta no corresponde a esta solicitud.");
  return confirmado;
}

export async function cancelarCita(id: string) {
  z.uuid().parse(id);
  const { error } = await supabase.rpc("cancelar_cita_anfitrion", { p_id: id });
  if (error) throw error;
}
