import { supabase } from "../lib/supabase";

/**
 * Operadores/administradores globales (ver docs/plan-panel-administrativo-web.md,
 * punto 4): mismo modelo que contratistas.ts -- dar de baja desde acá los
 * deja sin acceso en TODOS los sitios, `sitio_id` queda como dato
 * informativo ("de dónde es"), no como filtro. RLS: SELECT/UPDATE global
 * para cualquier sesión autenticada (migración crea_usuarios_globales),
 * igual que contratistas/empresas. ROOT nunca llega acá -- queda 100%
 * local a cada dispositivo (ver enviar_usuario en src/nube/sincronizacion.rs).
 */
export interface Usuario {
  id: string;
  sitio_id: string;
  sitio_nombre: string | null;
  cedula: string;
  nombre: string;
  rol: "ADMINISTRADOR" | "OPERADOR";
  activo: boolean;
}

interface FilaCruda {
  id: string;
  sitio_id: string;
  sitios: { nombre: string } | null;
  cedula: string;
  nombre: string;
  rol: "ADMINISTRADOR" | "OPERADOR";
  activo: boolean;
}

export async function listarUsuarios(): Promise<Usuario[]> {
  const { data, error } = await supabase
    .from("usuarios")
    .select("id, sitio_id, cedula, nombre, rol, activo, sitios(nombre)")
    .order("nombre")
    .returns<FilaCruda[]>();

  if (error) throw new Error(error.message);
  return data.map(({ sitios, ...resto }) => ({ ...resto, sitio_nombre: sitios?.nombre ?? null }));
}

export async function actualizarActivoUsuario(id: string, activo: boolean): Promise<void> {
  const { error } = await supabase.from("usuarios").update({ activo }).eq("id", id);
  if (error) throw new Error(error.message);
}
