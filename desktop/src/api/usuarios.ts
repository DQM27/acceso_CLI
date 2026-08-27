import { invoke } from "@tauri-apps/api/core";
import type { RolUsuario } from "./autenticacion";

// Espejo de comandos/usuarios.rs y dto/usuarios.rs. RolUsuario ya lo exporta
// autenticacion.ts (mismo tipo, un solo lugar de origen vía el barrel).

export interface UsuarioResumen {
  id: number;
  cedula: string;
  nombre: string;
  rol: RolUsuario;
  activo: boolean;
}

export interface FiltroUsuarios {
  texto?: string;
}

export interface DatosCrearUsuario {
  cedula: string;
  nombre: string;
  password: string;
  rol: RolUsuario;
  activo: boolean;
}

export interface DatosActualizarUsuario {
  cedula: string;
  nombre: string;
  rol: RolUsuario;
  activo: boolean;
}

export function buscarUsuarios(filtro: FiltroUsuarios): Promise<UsuarioResumen[]> {
  return invoke("buscar_usuarios", { filtro });
}

export function crearUsuario(datos: DatosCrearUsuario): Promise<number> {
  return invoke("crear_usuario", { datos });
}

export function actualizarUsuario(id: number, datos: DatosActualizarUsuario): Promise<void> {
  return invoke("actualizar_usuario", { id, datos });
}

export function cambiarPasswordUsuario(id: number, password: string): Promise<void> {
  return invoke("cambiar_password_usuario", { id, password });
}
