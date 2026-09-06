import { supabase } from "../lib/supabase";

/**
 * Usuarios globales (ver docs/plan-panel-administrativo-web.md, punto 4):
 * mismo modelo que contratistas.ts -- dar de baja desde acá los deja sin
 * acceso en TODOS los sitios, `sitio_id` queda como dato informativo ("de
 * dónde es"), no como filtro. RLS: SELECT/UPDATE global para cualquier
 * sesión autenticada (migración crea_usuarios_globales), igual que
 * contratistas/empresas -- INSERT sólo para admin_global (migración
 * admin_global_crea_usuarios) o un dispositivo creando en su propio sitio.
 * ROOT viaja acá también desde 2026-09-06 (migración
 * permite_root_en_usuarios_globales) -- antes quedaba 100% local a cada
 * dispositivo.
 */
export type RolUsuario = "ROOT" | "ADMINISTRADOR" | "OPERADOR";

export interface Usuario {
  id: string;
  sitio_id: string;
  sitio_nombre: string | null;
  cedula: string;
  nombre: string;
  rol: RolUsuario;
  activo: boolean;
}

interface FilaCruda {
  id: string;
  sitio_id: string;
  sitios: { nombre: string } | null;
  cedula: string;
  nombre: string;
  rol: RolUsuario;
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

/** Para elegir `sitio_id` al crear -- hoy sólo existe "Brisas", pero no
 * hay que asumirlo hardcodeado en el formulario. */
export async function listarSitios(): Promise<{ id: string; nombre: string }[]> {
  const { data, error } = await supabase.from("sitios").select("id, nombre").order("nombre");
  if (error) throw new Error(error.message);
  return data;
}

/**
 * Sin contraseña a propósito -- entra con el centinela `SIN_PASSWORD_LOCAL`
 * (ver `src/services/password.rs`), el primer dispositivo donde esta
 * cédula inicia sesión es el que la fija de verdad. `rol` se limita a
 * ADMINISTRADOR/OPERADOR desde acá -- dar de alta un ROOT nuevo sigue
 * siendo una decisión aparte, no algo que se banalice desde un formulario
 * web (sigue disponible por CLI/TUI: `crear_root_inicial`).
 */
export async function crearUsuario(datos: {
  sitio_id: string;
  cedula: string;
  nombre: string;
  rol: "ADMINISTRADOR" | "OPERADOR";
}): Promise<void> {
  const { error } = await supabase.from("usuarios").insert(datos);
  if (error) {
    if (error.code === "23505") throw new Error("Ya existe un usuario con esa cédula.");
    throw new Error(error.message);
  }
}
