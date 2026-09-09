import { z } from "./lib/validacion";
import { supabase } from "./lib/supabase";
import {
  citaEsquema,
  esquemaNuevaCita,
  sitioEsquema,
  MAX_SITIOS,
} from "./dominio";
import type { FiltroEstado, FormularioCita } from "./dominio";
import { hoyCostaRica } from "./fecha";

export const TAMANO_PAGINA = 12;
export const CAMPOS_CITA =
  "id,anfitrion_correo,motivo,fecha_desde,fecha_hasta,estado,created_at,cita_visitantes(id,nombre,cedula,empresa,placa_vehiculo),cita_sitios(sitio_id,sitios(id,nombre,direccion))";

export function mensajeError(error: unknown): string {
  const codigo =
    typeof error === "object" && error !== null && "code" in error
      ? error.code
      : undefined;
  if (codigo === "PGRST202" || codigo === "42P17")
    return "El servicio de visitas aún no está disponible. Intentá más tarde o contactá a administración.";
  if (codigo === "42501" || codigo === "PGRST301")
    return "No tenés permiso para realizar esta acción. Verificá tu sesión.";
  if (codigo === "23505")
    return "Esta solicitud ya existe. Actualizá tus citas antes de intentarlo otra vez.";
  return "No pudimos completar la solicitud. Revisá tu conexión e intentá de nuevo.";
}

export async function listarSitios(signal?: AbortSignal) {
  let consulta = supabase
    .from("sitios")
    .select("id,nombre,direccion")
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

export async function listarCitas(
  correo: string,
  filtro: FiltroEstado,
  pagina: number,
  signal?: AbortSignal,
) {
  z.email().parse(correo);
  z.number().int().nonnegative().max(100_000).parse(pagina);
  let consulta = supabase
    .from("citas")
    .select(CAMPOS_CITA)
    .eq("anfitrion_correo", correo);
  if (filtro === "VIGENTE")
    consulta = consulta
      .eq("estado", "VIGENTE")
      .gte("fecha_hasta", hoyCostaRica());
  if (filtro === "VENCIDA")
    consulta = consulta
      .eq("estado", "VIGENTE")
      .lt("fecha_hasta", hoyCostaRica());
  if (filtro === "CANCELADA") consulta = consulta.eq("estado", "CANCELADA");
  consulta = consulta
    .order("fecha_desde", { ascending: false })
    .order("id", { ascending: false })
    .range(pagina * TAMANO_PAGINA, (pagina + 1) * TAMANO_PAGINA);
  if (signal) consulta = consulta.abortSignal(signal);
  const { data, error } = await consulta;
  if (error) throw error;
  const citas = z.array(citaEsquema).parse(data);
  return {
    citas: citas.slice(0, TAMANO_PAGINA),
    hayMas: citas.length > TAMANO_PAGINA,
  };
}

export async function crearCita(
  id: string,
  formulario: FormularioCita,
  fechaValidacion = hoyCostaRica(),
) {
  z.uuid().parse(id);
  // El reintento de una respuesta perdida conserva la fecha del primer envío:
  // cruzar medianoche no debe impedir recuperar una cita ya confirmada en la base.
  const datos = esquemaNuevaCita(fechaValidacion).parse(formulario);
  const { data, error } = await supabase.rpc("crear_cita_anfitrion", {
    p_id: id,
    p_fecha_desde: datos.fecha_desde,
    p_fecha_hasta: datos.fecha_hasta,
    p_motivo: datos.motivo,
    p_sitios: datos.sitios,
    p_visitantes: datos.visitantes,
  });
  if (error) throw error;
  const confirmado = z.uuid().parse(data);
  if (confirmado !== id)
    throw new Error("La respuesta no corresponde a esta solicitud.");
  return confirmado;
}

export async function cancelarCita(id: string, correo: string) {
  z.uuid().parse(id);
  z.email().parse(correo);
  const { data, error } = await supabase
    .from("citas")
    .update({ estado: "CANCELADA" })
    .eq("id", id)
    .eq("anfitrion_correo", correo)
    .eq("estado", "VIGENTE")
    .select("id")
    .single();
  if (error) throw error;
  if (data?.id !== id) throw new Error("No se confirmó la cancelación.");
}
