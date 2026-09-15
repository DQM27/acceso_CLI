import { invoke } from "@tauri-apps/api/core";
import { solicitarSincronizacionNube } from "../eventosNube";

// Espejo de comandos/rutas.rs y dto/rutas.rs — ver también
// src/application/rutas.rs y src/models/{vehiculo_ruta,encargado_ruta,salida_ruta}.rs
// del núcleo. Dominio de "Control de rutas"
// (docs/planes-implementados/plan-control-rutas.md).

export type ResultadoSalidaRuta = "Permitido" | "PermitidoConAutorizacion";

export interface VehiculoRuta {
  id: number;
  numero_unidad: string | null;
  placa: string;
  activo: boolean;
}

export interface EncargadoRuta {
  id: number;
  codigo_empleado: string;
  nombre: string;
  /** Nunca se captura del lado de escritorio -- ver dto/rutas.rs. */
  cedula: string | null;
  activo: boolean;
}

export interface DatosVehiculoRuta {
  numero_unidad: string | null;
  placa: string;
  activo: boolean;
}

export interface DatosEncargadoRuta {
  codigo_empleado: string;
  nombre: string;
  activo: boolean;
}

export function listarVehiculosRuta(): Promise<VehiculoRuta[]> {
  return invoke("listar_vehiculos_ruta");
}

export async function crearVehiculoRuta(datos: DatosVehiculoRuta): Promise<number> {
  const id = await invoke<number>("crear_vehiculo_ruta", { datos });
  solicitarSincronizacionNube();
  return id;
}

export async function actualizarVehiculoRuta(id: number, datos: DatosVehiculoRuta): Promise<void> {
  await invoke("actualizar_vehiculo_ruta", { id, datos });
  solicitarSincronizacionNube();
}

export function listarEncargadosRuta(): Promise<EncargadoRuta[]> {
  return invoke("listar_encargados_ruta");
}

export async function crearEncargadoRuta(datos: DatosEncargadoRuta): Promise<number> {
  const id = await invoke<number>("crear_encargado_ruta", { datos });
  solicitarSincronizacionNube();
  return id;
}

export async function actualizarEncargadoRuta(
  id: number,
  datos: DatosEncargadoRuta,
): Promise<void> {
  await invoke("actualizar_encargado_ruta", { id, datos });
  solicitarSincronizacionNube();
}

/** Fila para la pantalla "Rutas activas" -- análoga a
 * `MovimientoVisitaActivoResumen`. */
export interface SalidaRutaActivaResumen {
  id: number;
  vehiculo_placa: string;
  vehiculo_numero_unidad: string | null;
  encargado_nombre: string;
  numero_ruta: string;
  sub_numero: number;
  numero_documento: string;
  /** `YYYY-MM-DD`. */
  fecha_documento: string;
  resultado: ResultadoSalidaRuta;
  /** ISO 8601 (UTC) — convertir con `new Date(...)` antes de mostrar. */
  fecha_hora_salida: string;
  usuario_salida_nombre: string;
}

/** Espejo de `SolicitudSalidaRutaEntrada` (dto/rutas.rs) --
 * `usuario_salida_id`/`fecha_hora_salida` no viajan: el núcleo los fija con
 * el actor y el reloj validados de la transacción, nunca con lo que mande
 * el webview. */
export interface SolicitudSalidaRuta {
  vehiculo_placa: string;
  vehiculo_numero_unidad: string | null;
  encargado_nombre: string;
  encargado_codigo_empleado: string | null;
  numero_ruta: string;
  sub_numero: number;
  numero_documento: string;
  /** `YYYY-MM-DD`. */
  fecha_documento: string;
  tiene_correo_autorizacion: boolean;
}

export interface RetornoSalidaRuta {
  /** ISO 8601 (UTC). */
  fecha_hora: string;
  usuario_id: number;
}

/** Registro completo -- lo usa la pantalla de detalle/confirmación de
 * retorno, a diferencia de `SalidaRutaActivaResumen` (fila de la grilla). */
export interface SalidaRuta {
  id: number;
  vehiculo_id: number | null;
  vehiculo_placa: string;
  vehiculo_numero_unidad: string | null;
  encargado_id: number | null;
  encargado_nombre: string;
  numero_ruta: string;
  sub_numero: number;
  numero_documento: string;
  /** `YYYY-MM-DD`. */
  fecha_documento: string;
  resultado: ResultadoSalidaRuta;
  /** ISO 8601 (UTC). */
  fecha_hora_salida: string;
  usuario_salida_id: number;
  retorno: RetornoSalidaRuta | null;
}

export async function registrarSalidaRuta(solicitud: SolicitudSalidaRuta): Promise<number> {
  const id = await invoke<number>("registrar_salida_ruta", { solicitud });
  solicitarSincronizacionNube();
  return id;
}

export async function registrarRetornoRuta(salidaId: number): Promise<void> {
  await invoke("registrar_retorno_ruta", { salidaId });
  solicitarSincronizacionNube();
}

export function listarRutasActivas(): Promise<SalidaRutaActivaResumen[]> {
  return invoke("listar_rutas_activas");
}

export function buscarSalidaRuta(id: number): Promise<SalidaRuta | null> {
  return invoke("buscar_salida_ruta", { id });
}
