import { invoke } from "@tauri-apps/api/core";
import { solicitarSincronizacionNube } from "../eventosNube";

// Espejo de comandos/proveedores.rs y dto/proveedores.rs — ver también
// src/application/proveedores.rs y src/models/{empresa_proveedor,registro_ingreso_proveedor}.rs
// del núcleo. Dominio de "Control de proveedores"
// (docs/features-futuras/plan-control-proveedores.md).

export interface EmpresaProveedor {
  id: number;
  nombre: string;
  activo: boolean;
}

/** Fila para la pantalla "Activos" -- análoga a `MovimientoVisitaActivoResumen`
 * y `SalidaRutaActivaResumen`. */
export interface ProveedorActivoResumen {
  id: number;
  cedula: string;
  nombre: string;
  empresa_nombre: string;
  placa: string | null;
  gafete_numero: number;
  /** ISO 8601 (UTC) — convertir con `new Date(...)` antes de mostrar. */
  fecha_hora_ingreso: string;
}

/** Espejo de `SolicitudIngresoProveedorEntrada` (dto/proveedores.rs). */
export interface SolicitudIngresoProveedor {
  cedula: string;
  nombre: string;
  empresa_id: number;
  placa: string | null;
  gafete_numero: number;
}

export function listarEmpresasProveedor(): Promise<EmpresaProveedor[]> {
  return invoke("listar_empresas_proveedor");
}

export function buscarEmpresasProveedor(texto: string): Promise<EmpresaProveedor[]> {
  return invoke("buscar_empresas_proveedor", { texto });
}

export async function crearEmpresaProveedor(nombre: string): Promise<number> {
  const id = await invoke<number>("crear_empresa_proveedor", { nombre });
  solicitarSincronizacionNube();
  return id;
}

export async function establecerEmpresaProveedorActiva(
  id: number,
  activa: boolean,
): Promise<void> {
  await invoke("establecer_empresa_proveedor_activa", { id, activa });
  solicitarSincronizacionNube();
}

export async function registrarIngresoProveedor(
  solicitud: SolicitudIngresoProveedor,
): Promise<number> {
  const id = await invoke<number>("registrar_ingreso_proveedor", { solicitud });
  solicitarSincronizacionNube();
  return id;
}

export async function registrarSalidaProveedor(id: number): Promise<void> {
  await invoke("registrar_salida_proveedor", { id });
  solicitarSincronizacionNube();
}

export function listarProveedoresActivos(): Promise<ProveedorActivoResumen[]> {
  return invoke("listar_proveedores_activos");
}
