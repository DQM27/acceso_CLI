import { invoke } from "@tauri-apps/api/core";
import { solicitarSincronizacionNube } from "../eventosNube";
import { cerrarIngresoProveedorRemoto, listarIngresosProveedorRemotos } from "./nube";
import type { IngresoProveedorRemoto } from "./nube";

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

/** Para la grilla de administración -- trae activas e inactivas, así se
 * puede reactivar una. `listarEmpresasProveedorSeleccionables` es la
 * contraparte para un selector de wizard (elegir empresa para un ingreso
 * nuevo), donde una empresa inactiva nunca es una opción válida -- esa
 * decisión la toma el núcleo (`EmpresaProveedorService`), no quien llama. */
export function listarEmpresasProveedor(): Promise<EmpresaProveedor[]> {
  return invoke("listar_empresas_proveedor");
}

export function listarEmpresasProveedorSeleccionables(): Promise<EmpresaProveedor[]> {
  return invoke("listar_empresas_proveedor_seleccionables");
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

// ---- Local + remoto fusionados -- mismo criterio que api/activos.ts para
// contratistas: un ingreso abierto por el OTRO dispositivo del mismo sitio
// nunca vive en `registro_ingresos_proveedor` de este, sólo en la caché
// `ingresos_proveedor_remotos` -- se fusionan acá para que la pantalla
// "Proveedores" muestre y pueda cerrar ambos sin distinguir de dónde vino
// cada fila. ----

export interface FilaProveedorLocal extends ProveedorActivoResumen {
  origen: "local";
}

export interface FilaProveedorRemota {
  origen: "remoto";
  uuid_remoto: string;
  id: null;
  cedula: string;
  nombre: string;
  empresa_nombre: string;
  placa: string | null;
  gafete_numero: number;
  fecha_hora_ingreso: string;
}

export type FilaProveedorActiva = FilaProveedorLocal | FilaProveedorRemota;

function filaProveedorDesdeLocal(item: ProveedorActivoResumen): FilaProveedorActiva {
  return { ...item, origen: "local" };
}

function filaProveedorDesdeRemoto(remoto: IngresoProveedorRemoto): FilaProveedorActiva {
  return {
    origen: "remoto",
    uuid_remoto: remoto.uuid,
    id: null,
    cedula: remoto.cedula,
    nombre: remoto.nombre,
    empresa_nombre: remoto.empresa_nombre,
    placa: remoto.placa,
    gafete_numero: remoto.gafete_numero,
    fecha_hora_ingreso: remoto.hora_entrada,
  };
}

/** Clave estable para listas de React (`key`) y grillas -- `id` es `null`
 * en una remota, así que no alcanza solo. */
export function claveFilaProveedorActiva(fila: FilaProveedorActiva): string {
  return fila.origen === "local" ? `local-${fila.id}` : `remoto-${fila.uuid_remoto}`;
}

export async function listarTodosLosProveedoresActivos(): Promise<FilaProveedorActiva[]> {
  const [locales, remotos] = await Promise.all([
    listarProveedoresActivos(),
    listarIngresosProveedorRemotos(),
  ]);
  return [...locales.map(filaProveedorDesdeLocal), ...remotos.map(filaProveedorDesdeRemoto)];
}

/** Local: cierra en `registro_ingresos_proveedor` (este dispositivo).
 * Remota: cierra directo contra la nube (`cerrarIngresoProveedorRemoto`) --
 * nunca toca el historial local, esa fila no es -- ni fue -- de este
 * dispositivo. */
export async function cerrarFilaProveedorActiva(fila: FilaProveedorActiva): Promise<void> {
  if (fila.origen === "local") {
    await registrarSalidaProveedor(fila.id);
  } else {
    await cerrarIngresoProveedorRemoto(fila.uuid_remoto);
  }
}

// ---- Historial (exclusivo de escritorio) ----

/** Espejo de `comandos::proveedores::HistorialIngresoProveedorRemoto` -- un
 * ingreso de proveedor (abierto o cerrado) leído de la caché local
 * `historial_ingresos_proveedor_sitio`, la misma que sincroniza
 * `nube::recibir_historial_ingresos_proveedor_del_sitio`. Sólo se llena en
 * PC -- el celular no trae esto (mismo criterio que
 * `MovimientoHistorialVisitaRemoto`, ver `api/citas.ts`), así que este
 * comando ni siquiera existe del lado móvil. */
export interface HistorialIngresoProveedorRemoto {
  uuid: string;
  cedula: string;
  nombre: string;
  empresa_nombre: string | null;
  placa: string | null;
  gafete_numero: number | null;
  /** ISO 8601 (UTC). */
  fecha_hora_ingreso: string;
  fecha_hora_salida: string | null;
  usuario_ingreso_nombre: string | null;
  usuario_salida_nombre: string | null;
}

export function listarHistorialIngresosProveedorSitio(
  desde?: string,
  hasta?: string,
): Promise<HistorialIngresoProveedorRemoto[]> {
  return invoke("listar_historial_ingresos_proveedor_sitio", {
    desde: desde ?? null,
    hasta: hasta ?? null,
  });
}
