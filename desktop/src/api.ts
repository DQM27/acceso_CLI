import { invoke } from "@tauri-apps/api/core";

// Espejo en TypeScript de los tipos que expone control_acceso vía serde
// (ver src/models/usuario.rs, src/services/autenticacion_service.rs,
// src/database/queries/contratistas.rs). Si el núcleo cambia esos structs,
// este archivo hay que actualizarlo a mano — no hay generación automática
// todavía.

export type RolUsuario = "Root" | "Administrador" | "Operador";

export interface UsuarioSesion {
  id: number;
  cedula: string;
  nombre: string;
  rol: RolUsuario;
}

export type TipoIngreso = "Praind" | "InHouse" | "PorCorreo" | "Swat";

export interface ContratistaResumen {
  id: number;
  empresa_id: number;
  cedula: string;
  nombre: string;
  empresa_nombre: string;
  tipo_ingreso: TipoIngreso;
  fecha_vencimiento_praind: string | null;
  es_personal_ruta: boolean;
  tiene_acceso: boolean;
  tiene_ingreso_activo: boolean;
}

export interface PaginaContratistas {
  items: ContratistaResumen[];
  total: number;
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

export function buscarContratistas(texto: string): Promise<PaginaContratistas> {
  return invoke("buscar_contratistas", { texto });
}
