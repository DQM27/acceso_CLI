import { supabase } from "../lib/supabase";

/**
 * Contratistas globales (ver docs/plan-panel-administrativo-web.md,
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
    .range(0, LIMITE_CONTRATISTAS - 1)
    .returns<Contratista[]>();

  if (error) throw new Error(error.message);
  return { filas: data, truncado: count !== null && count > data.length };
}

export async function actualizarAccesoContratista(id: string, activo: boolean): Promise<void> {
  const { error } = await supabase.from("contratistas").update({ activo }).eq("id", id);
  if (error) throw new Error(error.message);
}
