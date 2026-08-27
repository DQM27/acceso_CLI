import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/contratistas.rs y dto.rs (los tipos de filtro/edición
// son los DTO de frontera, no el FiltroContratistas/DatosActualizacionContratista
// reales del núcleo — esos tienen tipos como Igualdad<T> que no tiene sentido
// exponerle al webview tal cual).

export type TipoIngreso = "Praind" | "InHouse" | "PorCorreo" | "Swat";
export const TIPOS_INGRESO: TipoIngreso[] = ["Praind", "InHouse", "PorCorreo", "Swat"];

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

export type EstadoPraind = "vencido" | "proximo" | "sin_fecha";

export interface FiltroContratistas {
  texto?: string;
  empresa_id?: number;
  tipos?: TipoIngreso[];
  praind?: EstadoPraind;
  personal_ruta?: boolean;
  tiene_acceso?: boolean;
}

export interface DatosContratista {
  cedula: string;
  nombre: string;
  empresa_id: number;
  tipo_ingreso: TipoIngreso;
  fecha_vencimiento_praind: string | null;
  es_personal_ruta: boolean;
  tiene_acceso: boolean;
}

/// Espejo de Contratista::requiere_praind() (src/models/contratista.rs) — el
/// core es quien manda esta regla, acá sólo se replica para decidir cuándo
/// mostrar/exigir el campo de fecha en el formulario. La validación real
/// sigue pasando por el backend de todos modos.
export function requierePraind(datos: {
  es_personal_ruta: boolean;
  tipo_ingreso: TipoIngreso;
}): boolean {
  return (
    datos.es_personal_ruta ||
    datos.tipo_ingreso === "Praind" ||
    datos.tipo_ingreso === "InHouse"
  );
}

export function buscarContratistas(filtro: FiltroContratistas): Promise<PaginaContratistas> {
  return invoke("buscar_contratistas", { filtro });
}

export function crearContratista(datos: DatosContratista): Promise<number> {
  return invoke("crear_contratista", { datos });
}

export function actualizarContratista(id: number, datos: DatosContratista): Promise<void> {
  return invoke("actualizar_contratista", { id, datos });
}
