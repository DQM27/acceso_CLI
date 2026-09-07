import { z } from "zod";
import { supabase } from "../lib/supabase";

/**
 * Usuarios globales (ver docs/plan-panel-administrativo-web.md, punto 4):
 * un usuario/operador no pertenece a un sitio -- dar de baja desde acá lo
 * deja sin acceso en TODOS a la vez. `sitio_id` en la tabla real queda como
 * dato de procedencia (qué dispositivo lo creó, o a qué sitio quedó
 * asociado un alta desde el panel), pero no se pide para esta lista -- no
 * aporta nada para decidir nada acá (mismo criterio que contratistas.ts).
 * RLS: SELECT/UPDATE global para cualquier sesión autenticada (migración
 * crea_usuarios_globales) -- INSERT sólo para admin_global (migración
 * admin_global_crea_usuarios) o un dispositivo creando en su propio sitio.
 * ROOT viaja acá también desde 2026-09-06 (migración
 * permite_root_en_usuarios_globales) -- antes quedaba 100% local a cada
 * dispositivo.
 */
export type RolUsuario = "ROOT" | "ADMINISTRADOR" | "OPERADOR";

export interface Usuario {
  id: string;
  cedula: string;
  nombre: string;
  rol: RolUsuario;
  activo: boolean;
}

export interface ResultadoUsuarios {
  filas: Usuario[];
  /** Ver el mismo campo en `ResultadoHistorial` (`api/historial.ts`) --
   * misma razón: AG Grid corre client-side (`componentes/Tabla.tsx`), sin
   * este tope la tabla completa crece sin cota junto con el catálogo real. */
  truncado: boolean;
}

// Ver el mismo criterio en contratistas.ts -- valida en runtime la forma
// real de lo que devuelve Supabase.
const filaUsuarioEsquema = z.object({
  id: z.string(),
  cedula: z.string(),
  nombre: z.string(),
  rol: z.enum(["ROOT", "ADMINISTRADOR", "OPERADOR"]),
  activo: z.boolean(),
});

// Válvula de seguridad, no paginación real -- muy por encima de cualquier
// catálogo de usuarios/operadores real de un solo sitio.
const LIMITE_USUARIOS = 10_000;

export async function listarUsuarios(): Promise<ResultadoUsuarios> {
  const { data, error, count } = await supabase
    .from("usuarios")
    .select("id, cedula, nombre, rol, activo", { count: "exact" })
    .order("nombre")
    .range(0, LIMITE_USUARIOS - 1);

  if (error) throw new Error(error.message);
  return { filas: z.array(filaUsuarioEsquema).parse(data), truncado: count !== null && count > data.length };
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
