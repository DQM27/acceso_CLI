import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";

/**
 * Contratistas globales (ver docs/planes-implementados/plan-panel-administrativo-web.md,
 * "Modelo de datos"): un contratista no pertenece a un sitio -- puede
 * entrar en cualquier unidad operativa salvo que se le niegue el acceso, y
 * esa baja se ve en TODOS los sitios a la vez. `sitio_id` en la tabla real
 * queda como dato de procedencia (qué dispositivo lo dio de alta), pero
 * el panel ni siquiera lo pide -- no aporta nada para decidir nada acá.
 * RLS: `admin_global` (`es_admin_global()`) O cualquier dispositivo
 * autenticado (JWT con `sitio_id` -- móvil/escritorio de cualquier sitio,
 * necesario para que `recibir_catalogo_del_sitio` sincronice el catálogo
 * completo, ver `src/nube/sincronizacion.rs`). Ojo: NO es "sólo
 * admin_global" -- ese fue el estado original
 * (`admin_global_gestiona_contratistas`), reemplazado por la política
 * "(global)" en `globaliza_contratistas_y_empresas.sql`, y acotado de
 * nuevo (pero a device-o-admin_global, no sólo admin_global) en
 * `cierra_acceso_global_a_cuentas_sin_dispositivo_ni_admin.sql` -- ver esa
 * migración para el hallazgo de seguridad que la motivó (cualquier cuenta
 * de Google, ni siquiera un dispositivo, tenía el mismo acceso antes de
 * ese fix).
 */
export interface Contratista {
  id: string;
  identificacion: string | null;
  nombre: string;
  empresa_nombre: string | null;
  tipo_ingreso: string | null;
  fecha_vencimiento_praind: string | null;
  es_personal_ruta: boolean | null;
  activo: boolean;
}

export interface ResultadoContratistas {
  filas: Contratista[];
  /** Ver el mismo campo en `ResultadoHistorial` (`api/historial.ts`) --
   * misma razón: AG Grid corre client-side (`componentes/Tabla.tsx`), sin
   * este tope la tabla completa crece sin cota junto con el catálogo real. */
  truncado: boolean;
}

// Valida en runtime la forma real de lo que devuelve Supabase -- sin esto,
// un cambio de contrato del lado del backend (columna renombrada, tipo
// cambiado) pasaba en silencio hasta romper algo mucho más abajo, con un
// mensaje de error que no señalaba la causa real.
const filaContratistaEsquema = z.object({
  id: z.string(),
  identificacion: z.string().nullable(),
  nombre: z.string(),
  empresa_nombre: z.string().nullable(),
  tipo_ingreso: z.string().nullable(),
  fecha_vencimiento_praind: z.string().nullable(),
  es_personal_ruta: z.boolean().nullable(),
  activo: z.boolean(),
});

// Válvula de seguridad, no paginación real -- muy por encima de cualquier
// catálogo de contratistas real de un solo sitio.
const LIMITE_CONTRATISTAS = 10_000;

export async function listarContratistas(): Promise<ResultadoContratistas> {
  const { data, error, count } = await supabase
    .from("contratistas")
    .select(
      "id, identificacion, nombre, empresa_nombre, tipo_ingreso, " +
        "fecha_vencimiento_praind, es_personal_ruta, activo",
      { count: "exact" },
    )
    .order("nombre")
    .range(0, LIMITE_CONTRATISTAS - 1);

  if (error) throw new Error(error.message);
  return { filas: z.array(filaContratistaEsquema).parse(data), truncado: count !== null && count > data.length };
}

export async function actualizarAccesoContratista(id: string, activo: boolean): Promise<void> {
  const { error } = await supabase.from("contratistas").update({ activo }).eq("id", id);
  if (error) throw new Error(error.message);
}

// --- Alta desde el panel --------------------------------------------------
//
// Darlo de alta como contratista con el acceso apagado es la forma de negar el
// acceso a alguien que nunca fue contratista (un proveedor, por ejemplo): ver
// docs/features-futuras/plan-veto-por-persona.md. La base sólo deja crear a un
// equipo, así que el panel usa funciones que exigen ser administrador del
// panel y aplican las mismas reglas del núcleo (cédula en forma única,
// nombre en mayúsculas, PRAIND según el tipo). Ver la migración
// `panel_crea_contratistas`. Los mensajes de error vienen ya en español.

export interface Empresa {
  id: string;
  nombre: string;
}

const empresaEsquema = z.object({ id: z.string(), nombre: z.string() });

/** Empresas activas (las de contratistas), para elegir en el alta. */
export async function listarEmpresas(): Promise<Empresa[]> {
  const { data, error } = await supabase
    .from("empresas")
    .select("id, nombre")
    .eq("activa", true)
    .order("nombre");
  if (error) throw new Error(error.message);
  return z.array(empresaEsquema).parse(data);
}

/** Crea la empresa, o devuelve la que ya existe con ese nombre (sin importar
 * tildes ni mayúsculas). */
export async function crearEmpresa(nombre: string): Promise<Empresa> {
  const { data, error } = await supabase.rpc("panel_crear_empresa", { p_nombre: nombre });
  if (error) throw new Error(error.message);
  return empresaEsquema.parse(data);
}

export type TipoIngreso = "PRAIND" | "IN_HOUSE" | "POR_CORREO" | "SWAT";

export interface DatosNuevoContratista {
  cedula: string;
  nombre: string;
  empresa_id: string;
  tipo_ingreso: TipoIngreso;
  /** "AAAA-MM-DD"; `null` si no aplica. */
  fecha_vencimiento_praind: string | null;
  /** `false` lo crea con el acceso denegado (el bloqueo). */
  con_acceso: boolean;
}

export async function crearContratista(datos: DatosNuevoContratista): Promise<Contratista> {
  const { data, error } = await supabase.rpc("panel_crear_contratista", {
    p_cedula: datos.cedula,
    p_nombre: datos.nombre,
    p_empresa_id: datos.empresa_id,
    p_tipo_ingreso: datos.tipo_ingreso,
    p_fecha_vencimiento_praind: datos.fecha_vencimiento_praind,
    p_con_acceso: datos.con_acceso,
  });
  if (error) throw new Error(error.message);
  return filaContratistaEsquema.parse(data);
}
