import { z } from "./lib/validacion";
import { supabase } from "./lib/supabase";
import {
  esquemaNuevaVisita,
  misVisitasFilaEsquema,
  sitioEsquema,
  visitanteAnteriorEsquema,
  MAX_SITIOS,
} from "./dominio";
import type { FormularioNuevaVisita } from "./dominio";

export const CAMPOS_MIS_VISITAS =
  "visita_id,sitio_id,sitio_nombre,anfitrion_id,tipo_visita,motivo,fecha_desde,fecha_hasta,hora_desde,hora_hasta,requiere_escolta,grupo_id,origen,visita_estado,invitado_id,visitante_id,tipo_documento,numero_documento,visitante_nombre,visitante_empresa,placa_vehiculo,invitado_estado,aprobado_por,aprobado_en,motivo_rechazo,ultima_entrada,ultima_salida,ultimo_gafete_numero";

export function mensajeError(error: unknown): string {
  const codigo =
    typeof error === "object" && error !== null && "code" in error
      ? error.code
      : undefined;
  if (codigo === "PGRST202" || codigo === "42P17")
    return "El servicio de visitas aún no está disponible. Pruebe más tarde o contacte a administración.";
  if (codigo === "42501" || codigo === "PGRST301")
    return "No tiene permiso para realizar esta acción. Verifique su sesión.";
  if (codigo === "23505")
    return "Esta solicitud ya existe. Actualice sus visitas antes de intentarlo de nuevo.";
  if (codigo === "54000")
    return "Demasiadas solicitudes en poco tiempo. Espere unos minutos e intente de nuevo.";
  return "No pudimos completar la solicitud. Revise su conexión e intente de nuevo.";
}

export async function listarSitios(signal?: AbortSignal) {
  let consulta = supabase
    .from("sitios")
    .select("id,nombre")
    .order("nombre")
    .order("id")
    .limit(MAX_SITIOS + 1);
  if (signal) consulta = consulta.abortSignal(signal);
  const { data, error } = await consulta;
  if (error) throw error;
  const sitios = z.array(sitioEsquema).parse(data);
  if (sitios.length > MAX_SITIOS)
    throw new Error("El catálogo excede el límite de esta versión.");
  return sitios;
}

/** Sitios donde el anfitrión que llama puede agendar visitas (tabla
 * anfitrion_sitios, sólo lectura desde acá -- alta/baja es V5 del panel). */
export async function listarSitiosDelAnfitrion(signal?: AbortSignal) {
  let consulta = supabase
    .from("anfitrion_sitios")
    .select("sitio_id,sitios(id,nombre)")
    .order("sitio_id");
  if (signal) consulta = consulta.abortSignal(signal);
  const { data, error } = await consulta;
  if (error) throw error;
  const filas = z
    .array(z.object({ sitio_id: z.uuid(), sitios: sitioEsquema.nullable() }))
    .parse(data);
  return filas
    .map((f) => f.sitios)
    .filter((s): s is NonNullable<typeof s> => s !== null);
}

/** Todas las filas de `mis_visitas` visibles para quien llama (RLS ya
 * acota a "mías" para un anfitrión) -- se agrupan en visitas del lado del
 * cliente, ver `agruparVisitas` en dominio.ts. */
export async function listarMisVisitas(signal?: AbortSignal) {
  let consulta = supabase
    .from("mis_visitas")
    .select(CAMPOS_MIS_VISITAS)
    .order("fecha_desde", { ascending: false })
    .order("visita_id", { ascending: false });
  if (signal) consulta = consulta.abortSignal(signal);
  const { data, error } = await consulta;
  if (error) throw error;
  return z.array(misVisitasFilaEsquema).parse(data);
}

export async function buscarVisitantesAnteriores(busqueda: string, signal?: AbortSignal) {
  let consulta = supabase.rpc("visitantes_anteriores_del_anfitrion", {
    p_busqueda: busqueda,
  });
  if (signal) consulta = consulta.abortSignal(signal);
  const { data, error } = await consulta;
  if (error) throw error;
  return z.array(visitanteAnteriorEsquema).parse(data);
}

export async function crearVisitas(
  grupoId: string,
  formulario: FormularioNuevaVisita,
  fechaValidacion?: string,
) {
  z.uuid().parse(grupoId);
  const datos = fechaValidacion
    ? esquemaNuevaVisita(fechaValidacion).parse(formulario)
    : esquemaNuevaVisita().parse(formulario);
  const { data, error } = await supabase.rpc("crear_visitas", {
    p_grupo_id: grupoId,
    p_sitios: datos.sitios,
    p_fecha_desde: datos.fecha_desde,
    p_fecha_hasta: datos.fecha_hasta,
    p_hora_desde: datos.hora_desde,
    p_hora_hasta: datos.hora_hasta,
    p_tipo_visita: datos.tipo_visita,
    p_motivo: datos.motivo,
    p_requiere_escolta: datos.requiere_escolta,
    p_invitados: datos.invitados,
  });
  if (error) throw error;
  return z.array(z.uuid()).parse(data);
}

export async function editarVisita(visitaId: string, formulario: FormularioNuevaVisita) {
  z.uuid().parse(visitaId);
  const datos = esquemaNuevaVisita().parse(formulario);
  const { error } = await supabase.rpc("editar_visita", {
    p_visita_id: visitaId,
    p_fecha_desde: datos.fecha_desde,
    p_fecha_hasta: datos.fecha_hasta,
    p_hora_desde: datos.hora_desde,
    p_hora_hasta: datos.hora_hasta,
    p_motivo: datos.motivo,
    p_requiere_escolta: datos.requiere_escolta,
    p_invitados: datos.invitados,
  });
  if (error) throw error;
}

export async function cancelarVisita(visitaId: string) {
  z.uuid().parse(visitaId);
  const { error } = await supabase.rpc("cancelar_visita", { p_visita_id: visitaId });
  if (error) throw error;
}

export async function duplicarVisita(
  visitaId: string,
  nuevoId: string,
  fechaDesde: string,
  fechaHasta: string,
) {
  z.uuid().parse(visitaId);
  z.uuid().parse(nuevoId);
  z.iso.date().parse(fechaDesde);
  z.iso.date().parse(fechaHasta);
  const { data, error } = await supabase.rpc("duplicar_visita", {
    p_visita_id: visitaId,
    p_nuevo_id: nuevoId,
    p_fecha_desde: fechaDesde,
    p_fecha_hasta: fechaHasta,
  });
  if (error) throw error;
  return z.uuid().parse(data);
}

export async function responderSolicitud(
  invitadoId: string,
  aprobar: boolean,
  motivoRechazo: string | null,
) {
  z.uuid().parse(invitadoId);
  const { error } = await supabase.rpc("responder_solicitud", {
    p_invitado_id: invitadoId,
    p_aprobar: aprobar,
    p_motivo_rechazo: motivoRechazo,
  });
  if (error) throw error;
}
