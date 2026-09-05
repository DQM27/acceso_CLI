import { supabase } from "../lib/supabase";
import type { RolAdminPanel } from "./index";

export interface AdministradorPanel {
  correo: string;
  rol: RolAdminPanel;
  creado_en: string;
}

/** Sólo lee/escribe `administradores_panel` -- ninguna pantalla llama a
 * `supabase` directo para esto, mismo criterio que `api/*.ts` en
 * `desktop/` (una sola capa por dominio). RLS (ver migración
 * `administradores_panel_gestion_admin_global`) es quien de verdad decide
 * si esto funciona o no; acá no hay chequeo de rol porque sería
 * redundante -- Postgres ya lo hace. */

export async function listarAdministradores(): Promise<AdministradorPanel[]> {
  const { data, error } = await supabase
    .from("administradores_panel")
    .select("correo, rol, creado_en")
    .order("creado_en", { ascending: false });

  if (error) throw new Error(error.message);
  return data;
}

export async function agregarAdministrador(
  correo: string,
  rol: RolAdminPanel,
): Promise<void> {
  const { error } = await supabase
    .from("administradores_panel")
    .insert({ correo: correo.trim().toLowerCase(), rol });

  if (error) {
    if (error.code === "23505") {
      throw new Error("Ese correo ya está en la lista.");
    }
    throw new Error(error.message);
  }
}

export async function eliminarAdministrador(correo: string): Promise<void> {
  const { error } = await supabase.from("administradores_panel").delete().eq("correo", correo);
  if (error) throw new Error(error.message);
}
