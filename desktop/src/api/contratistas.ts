import { invoke } from "@tauri-apps/api/core";
import { solicitarSincronizacionNube } from "../eventosNube";

// Espejo de comandos/contratistas.rs y dto.rs (los tipos de filtro/edición
// son los DTO de frontera, no el FiltroContratistas/DatosActualizacionContratista
// reales del núcleo — esos tienen tipos como Igualdad<T> que no tiene sentido
// exponerle al webview tal cual).

export type TipoIngreso = "Praind" | "InHouse" | "PorCorreo" | "Swat";
export const TIPOS_INGRESO: TipoIngreso[] = ["Praind", "InHouse", "PorCorreo", "Swat"];

/** Texto de un tipo de ingreso en las grillas: en mayúsculas y con las
 * palabras separadas ("IN HOUSE", no el nombre interno "InHouse") --
 * pedido del usuario 2026-09-23. Mismo texto que el Excel/PDF del núcleo
 * (`historial::exportacion::tipo_texto`). `null` (fila remota vieja) → "—". */
export function textoTipoIngreso(tipo: TipoIngreso | null): string {
  switch (tipo) {
    case "Praind":
      return "PRAIND";
    case "InHouse":
      return "IN HOUSE";
    case "PorCorreo":
      return "POR CORREO";
    case "Swat":
      return "SWAT";
    default:
      return "—";
  }
}

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

// Sólo texto: la grilla de Contratistas ya no lo usa (carga el universo
// completo y filtra/ordena del lado del cliente con AG Grid — ver
// Contratistas.tsx), pero el buscador en vivo de NuevoIngresoModal y
// GestionGafeteModal sigue necesitando una búsqueda de texto contra el
// servidor.
export interface FiltroContratistas {
  texto?: string;
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

export function buscarContratistas(filtro: FiltroContratistas = {}): Promise<PaginaContratistas> {
  return invoke("buscar_contratistas", { filtro });
}

export async function crearContratista(datos: DatosContratista): Promise<number> {
  const id = await invoke<number>("crear_contratista", { datos });
  solicitarSincronizacionNube();
  return id;
}

export async function actualizarContratista(id: number, datos: DatosContratista): Promise<void> {
  await invoke("actualizar_contratista", { id, datos });
  solicitarSincronizacionNube();
}
