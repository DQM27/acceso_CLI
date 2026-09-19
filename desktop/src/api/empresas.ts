import { invoke } from "@tauri-apps/api/core";
import { solicitarSincronizacionNube } from "../eventosNube";

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

/** Para la grilla de administración -- trae activas e inactivas.
 * `listarEmpresasSeleccionables` es la contraparte para el desplegable de
 * "Empresa" al crear/editar un contratista, donde una empresa desactivada
 * nunca es una opción válida -- esa decisión la toma el núcleo
 * (`EmpresaService`), no quien llama. */
export function listarEmpresas(): Promise<Empresa[]> {
  return invoke("listar_empresas");
}

export function listarEmpresasSeleccionables(): Promise<Empresa[]> {
  return invoke("listar_empresas_seleccionables");
}

export function buscarEmpresas(filtro: FiltroEmpresas): Promise<EmpresaResumen[]> {
  return invoke("buscar_empresas", { filtro });
}

export async function crearEmpresa(nombre: string): Promise<number> {
  const id = await invoke<number>("crear_empresa", { nombre });
  solicitarSincronizacionNube();
  return id;
}

export async function actualizarEmpresa(id: number, nombre: string): Promise<void> {
  await invoke("actualizar_empresa", { id, nombre });
  solicitarSincronizacionNube();
}

export async function establecerEmpresaActiva(id: number, activa: boolean): Promise<void> {
  await invoke("establecer_empresa_activa", { id, activa });
  solicitarSincronizacionNube();
}
