import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/empresas.rs.

export interface Empresa {
  id: number;
  nombre: string;
  activo: boolean;
}

export function listarEmpresas(): Promise<Empresa[]> {
  return invoke("listar_empresas");
}
