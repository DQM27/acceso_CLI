import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/exportacion.rs.

/** Espejo de `ColumnaTabla` (núcleo, `historial::exportacion`): título tal
 * cual se ve en la grilla y si el dato va alineado a la izquierda. */
export interface ColumnaDatosTabla {
  titulo: string;
  izquierda: boolean;
}

/** Guarda en `destino` un CSV ya armado del lado del cliente (ver
 * `exportarCsv` en `componentes/Tabla.tsx`). El backend agrega el BOM de
 * UTF-8 para que Excel lea bien las tildes. */
export function guardarCsv(destino: string, contenido: string): Promise<void> {
  return invoke("guardar_csv", { destino, contenido });
}

/** Excel de una tabla que armó la grilla (`datosVisibles` de `Tabla`),
 * con el mismo estilo que el Excel de Historial. Devuelve las filas
 * exportadas. */
export function exportarTablaXlsx(
  destino: string,
  columnas: ColumnaDatosTabla[],
  filas: string[][],
): Promise<number> {
  return invoke("exportar_tabla_xlsx", { destino, columnas, filas });
}

/** PDF de una tabla que armó la grilla, mismo documento que el PDF de
 * Historial. `titulo` va en el encabezado; `filtroDescripcion` debajo (ej.
 * "Filtro: Últimos 6 meses"). */
export function exportarTablaPdf(
  destino: string,
  titulo: string,
  filtroDescripcion: string,
  columnas: ColumnaDatosTabla[],
  filas: string[][],
): Promise<void> {
  return invoke("exportar_tabla_pdf", { destino, titulo, filtroDescripcion, columnas, filas });
}
