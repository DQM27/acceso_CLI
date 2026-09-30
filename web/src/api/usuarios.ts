import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";
import { invocar, esObjeto } from "./_invocar";

/**
 * Usuarios globales (ver docs/planes-implementados/plan-panel-administrativo-web.md, punto 4):
 * un usuario/operador no pertenece a un sitio -- dar de baja desde acá lo
 * deja sin acceso en TODOS a la vez (el aviso en vivo llega a todas las
 * unidades). La tabla no tiene `sitio_id`: ver la migración
 * `usuarios_globales_y_coherencia` (mismo criterio que contratistas.ts).
 * RLS: SELECT para dispositivos y admin_global; INSERT/UPDATE sólo para
 * admin_global.
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

export interface UsuarioCreado {
  usuario_id: string;
  cedula: string;
  /** Se muestra una sola vez -- el Edge Function no la vuelve a devolver
   * después de esta respuesta (ver docs/planes-implementados/plan-autenticacion-supabase-auth.md). */
  password_temporal: string;
}

function esUsuarioCreado(valor: unknown): valor is UsuarioCreado {
  return (
    esObjeto(valor) &&
    typeof valor.usuario_id === "string" &&
    typeof valor.cedula === "string" &&
    typeof valor.password_temporal === "string"
  );
}

function esPasswordReseteado(valor: unknown): valor is { usuario_id: string; password_temporal: string } {
  return esObjeto(valor) && typeof valor.password_temporal === "string";
}

/**
 * Da de alta al usuario global Y su cuenta de Supabase Auth con una
 * contraseña temporal de un solo uso (Edge Function `admin-create-usuario`
 * -- el hash vive en Auth, nunca en la tabla `usuarios`, ver
 * docs/planes-implementados/plan-autenticacion-supabase-auth.md). La persona entra con esa
 * temporal y la app la obliga a cambiarla antes de dejarla operar. `rol` se
 * limita a ADMINISTRADOR/OPERADOR desde acá -- dar de alta un ROOT nuevo
 * sigue siendo una decisión aparte, no algo que se banalice desde un
 * formulario web (sigue disponible por CLI/TUI: `crear_root_inicial`).
 */
export function crearUsuario(datos: {
  cedula: string;
  nombre: string;
  rol: "ADMINISTRADOR" | "OPERADOR";
}): Promise<UsuarioCreado> {
  return invocar("admin-create-usuario", esUsuarioCreado, datos);
}

/** "Olvidó la contraseña" -- genera una temporal nueva de un solo uso para
 * un usuario ya existente (Edge Function `admin-reset-password-usuario`). */
export function resetearPasswordUsuario(
  usuarioId: string,
): Promise<{ usuario_id: string; password_temporal: string }> {
  return invocar("admin-reset-password-usuario", esPasswordReseteado, { usuario_id: usuarioId });
}
