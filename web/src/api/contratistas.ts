import { z } from "../lib/validacion";
import { esObjeto, invocar } from "./_invocar";
import { supabase } from "../lib/supabase";

/**
 * Contratistas globales (ver docs/planes-implementados/plan-panel-administrativo-web.md,
 * "Modelo de datos"): un contratista no pertenece a un sitio -- puede
 * entrar en cualquier unidad operativa salvo que se le niegue el acceso, y
 * esa baja se ve en TODOS los sitios a la vez (el aviso en vivo sale por el
 * canal de todas las unidades). La tabla no tiene `sitio_id`: ver la
 * migración `catalogo_global_sin_unidad`.
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

/** Estado de la PRAIND según `panel_contratistas_estado`. */
export type EstadoPraind = "NO_REQUIERE" | "SIN_REGISTRO" | "VENCIDA" | "POR_VENCER" | "VIGENTE";

/** Resultado de las reglas de acceso (`verificar_acceso` del núcleo), tal
 * como lo calcula `panel_contratistas_estado`. */
export type EstadoAcceso =
  | "EMPRESA_INACTIVA"
  | "SIN_ACCESO"
  | "PRAIND_NO_REGISTRADO"
  | "PRAIND_VENCIDO"
  | "PERMITIDO_CON_ADVERTENCIA"
  | "PERMITIDO";

/** Fila de la lista: el contratista más su estado calculado en el servidor
 * (vista `panel_contratistas_estado`, migración `vistas_estado_y_adentro`)
 * y, si tiene un ingreso abierto, dónde y desde cuándo está adentro. */
export interface ContratistaConEstado extends Contratista {
  empresa_activa: boolean;
  requiere_praind: boolean;
  /** Días hasta el vencimiento (negativo si ya venció); `null` sin fecha. */
  dias_para_vencer: number | null;
  estado_praind: EstadoPraind;
  estado_acceso: EstadoAcceso;
  adentro_sitio_nombre: string | null;
  adentro_desde: string | null;
}

export interface ResultadoContratistas {
  filas: ContratistaConEstado[];
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

const filaContratistaConEstadoEsquema = filaContratistaEsquema.extend({
  empresa_activa: z.boolean(),
  requiere_praind: z.boolean(),
  dias_para_vencer: z.number().nullable(),
  estado_praind: z.enum(["NO_REQUIERE", "SIN_REGISTRO", "VENCIDA", "POR_VENCER", "VIGENTE"]),
  estado_acceso: z.enum([
    "EMPRESA_INACTIVA",
    "SIN_ACCESO",
    "PRAIND_NO_REGISTRADO",
    "PRAIND_VENCIDO",
    "PERMITIDO_CON_ADVERTENCIA",
    "PERMITIDO",
  ]),
  adentro_sitio_nombre: z.string().nullable(),
  adentro_desde: z.string().nullable(),
});

// Válvula de seguridad, no paginación real -- muy por encima de cualquier
// catálogo de contratistas real de un solo sitio.
const LIMITE_CONTRATISTAS = 10_000;

export async function listarContratistas(): Promise<ResultadoContratistas> {
  // La vista calcula el estado con las mismas reglas que el núcleo, así el
  // panel no las reimplementa. Cambiar el acceso sigue siendo sobre la tabla
  // (`actualizarAccesoContratista`).
  const { data, error, count } = await supabase
    .from("panel_contratistas_estado")
    .select(
      "id, identificacion, nombre, empresa_nombre, tipo_ingreso, " +
        "fecha_vencimiento_praind, es_personal_ruta, activo, empresa_activa, " +
        "requiere_praind, dias_para_vencer, estado_praind, estado_acceso, " +
        "adentro_sitio_nombre, adentro_desde",
      { count: "exact" },
    )
    .order("nombre")
    .range(0, LIMITE_CONTRATISTAS - 1);

  if (error) throw new Error(error.message);
  return {
    filas: z.array(filaContratistaConEstadoEsquema).parse(data),
    truncado: count !== null && count > data.length,
  };
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
// nombre en mayúsculas, PRAIND según el tipo). Se crean sin unidad: el
// catálogo es global. Ver las migraciones `panel_crea_contratistas` y
// `catalogo_global_sin_unidad`. Los mensajes de error vienen ya en español.

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

/** Alta por la Edge Function `admin-crear-contratista`: valida con las
 * reglas del núcleo (las mismas que el formulario, vía WebAssembly) y las que
 * necesitan datos (empresa, cédula repetida), y guarda. Si rechaza, el error
 * trae el mismo texto que muestran las apps. Reemplaza a la función SQL
 * `panel_crear_contratista` (ver docs/arquitectura/reglas-compartidas.md). */
export async function crearContratista(datos: DatosNuevoContratista): Promise<Contratista> {
  const fila = await invocar("admin-crear-contratista", esObjeto, { ...datos });
  return filaContratistaEsquema.parse(fila);
}
