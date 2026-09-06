import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/autenticacion.rs.

export type RolUsuario = "Root" | "Administrador" | "Operador";

export interface UsuarioSesion {
  id: number;
  cedula: string;
  nombre: string;
  rol: RolUsuario;
}

/// Espejo de `comandos::autenticacion::ErrorLogin` -- distinto de un string
/// plano para que la pantalla de login pueda mostrar el formulario de
/// "fijar contraseña" sin comparar el texto exacto del mensaje. `login`
/// rechaza la promesa con esto (no con un `Error` de JS), ver su uso en
/// `Login.tsx`.
export interface ErrorLogin {
  mensaje: string;
  sin_password_local: boolean;
}

export function esErrorLogin(error: unknown): error is ErrorLogin {
  return (
    typeof error === "object" &&
    error !== null &&
    "sin_password_local" in error &&
    "mensaje" in error
  );
}

export function requiereConfiguracionInicial(): Promise<boolean> {
  return invoke("requiere_configuracion_inicial");
}

export function login(cedula: string, password: string): Promise<UsuarioSesion> {
  return invoke("login", { cedula, password });
}

/// Completa el alta de contraseña de un usuario global que `login` marcó
/// con `sin_password_local` -- deja la sesión iniciada directo, igual que
/// un login exitoso.
export function fijarPasswordInicial(
  cedula: string,
  nuevaPassword: string,
): Promise<UsuarioSesion> {
  return invoke("fijar_password_inicial", { cedula, nuevaPassword });
}

export function cerrarSesion(): Promise<void> {
  return invoke("cerrar_sesion");
}
