import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/autenticacion.rs.

export type RolUsuario = "Root" | "Administrador" | "Operador";

export interface UsuarioSesion {
  id: number;
  cedula: string;
  nombre: string;
  rol: RolUsuario;
}

/// Espejo de `comandos::autenticacion::ErrorLogin` -- `login` rechaza la
/// promesa con esto (no con un `Error` de JS), ver su uso en `Login.tsx`.
export interface ErrorLogin {
  mensaje: string;
}

export function esErrorLogin(error: unknown): error is ErrorLogin {
  return typeof error === "object" && error !== null && "mensaje" in error;
}

/// Espejo de `comandos::autenticacion::ResultadoLogin` --
/// `debe_cambiar_password` fuerza el paso de cambio de contraseña antes de
/// dejar operar (usuario global recién creado en el panel, con la
/// temporal de un solo uso -- ver docs/plan-autenticacion-supabase-auth.md).
/// Siempre `false` para un login local (ROOT del arranque inicial).
export interface ResultadoLogin {
  sesion: UsuarioSesion;
  debe_cambiar_password: boolean;
}

export function requiereConfiguracionInicial(): Promise<boolean> {
  return invoke("requiere_configuracion_inicial");
}

export function login(cedula: string, password: string): Promise<ResultadoLogin> {
  return invoke("login", { cedula, password });
}

/// Cambio de contraseña contra Supabase Auth -- tanto el obligatorio tras
/// un primer login con temporal (`debe_cambiar_password`) como el
/// rutinario. Revalida `passwordActual` del lado del backend antes de
/// aceptar la nueva, no alcanza con tener la sesión abierta.
export function cambiarPasswordSupabase(
  passwordActual: string,
  passwordNueva: string,
): Promise<void> {
  return invoke("cambiar_password_supabase", { passwordActual, passwordNueva });
}

export function cerrarSesion(): Promise<void> {
  return invoke("cerrar_sesion");
}
