import { supabase } from "../lib/supabase";

/**
 * Contratistas globales (ver docs/plan-panel-administrativo-web.md,
 * "Modelo de datos"): dar de baja desde acá los deja sin acceso en TODOS
 * los sitios, no sólo el de origen -- `sitio_id` queda como dato
 * informativo ("de dónde es"), no como filtro de qué puede tocar el panel.
 * RLS: sólo `admin_global` (`es_admin_global()`, migración
 * `admin_global_gestiona_contratistas`).
 */
export interface Contratista {
  id: string;
  sitio_id: string;
  sitio_nombre: string | null;
  identificacion: string | null;
  nombre: string;
  empresa_nombre: string | null;
  tipo_ingreso: string | null;
  fecha_vencimiento_praind: string | null;
  es_personal_ruta: boolean | null;
  activo: boolean;
}

interface FilaCruda {
  id: string;
  sitio_id: string;
  sitios: { nombre: string } | null;
  identificacion: string | null;
  nombre: string;
  empresa_nombre: string | null;
  tipo_ingreso: string | null;
  fecha_vencimiento_praind: string | null;
  es_personal_ruta: boolean | null;
  activo: boolean;
}

export async function listarContratistas(): Promise<Contratista[]> {
  const { data, error } = await supabase
    .from("contratistas")
    .select(
      "id, sitio_id, identificacion, nombre, empresa_nombre, tipo_ingreso, " +
        "fecha_vencimiento_praind, es_personal_ruta, activo, sitios(nombre)",
    )
    .order("nombre")
    .returns<FilaCruda[]>();

  if (error) throw new Error(error.message);
  return data.map(({ sitios, ...resto }) => ({ ...resto, sitio_nombre: sitios?.nombre ?? null }));
}

export async function actualizarAccesoContratista(id: string, activo: boolean): Promise<void> {
  const { error } = await supabase.from("contratistas").update({ activo }).eq("id", id);
  if (error) throw new Error(error.message);
}
