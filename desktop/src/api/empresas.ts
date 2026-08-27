import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/empresas.rs y dto/empresas.rs.

export interface Empresa {
  id: number;
  nombre: string;
  activo: boolean;
}

export interface EmpresaResumen {
  id: number;
  nombre: string;
  contratistas: number;
  activo: boolean;
}

export interface FiltroEmpresas {
  texto?: string;
}

/** Lista completa sin filtro — la usan los desplegables de "Empresa" en otras pantallas. */
export function listarEmpresas(): Promise<Empresa[]> {
  return invoke("listar_empresas");
}

export function buscarEmpresas(filtro: FiltroEmpresas): Promise<EmpresaResumen[]> {
  return invoke("buscar_empresas", { filtro });
}

export function crearEmpresa(nombre: string): Promise<number> {
  return invoke("crear_empresa", { nombre });
}

export function actualizarEmpresa(id: number, nombre: string): Promise<void> {
  return invoke("actualizar_empresa", { id, nombre });
}

export function establecerEmpresaActiva(id: number, activa: boolean): Promise<void> {
  return invoke("establecer_empresa_activa", { id, activa });
}
