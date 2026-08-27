import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/autenticacion.rs.

export type RolUsuario = "Root" | "Administrador" | "Operador";

export interface UsuarioSesion {
  id: number;
  cedula: string;
  nombre: string;
  rol: RolUsuario;
}

export function requiereConfiguracionInicial(): Promise<boolean> {
  return invoke("requiere_configuracion_inicial");
}

export function login(cedula: string, password: string): Promise<UsuarioSesion> {
  return invoke("login", { cedula, password });
}

export function cerrarSesion(): Promise<void> {
  return invoke("cerrar_sesion");
}
