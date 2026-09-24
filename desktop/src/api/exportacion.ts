import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/exportacion.rs.

/** Guarda en `destino` un CSV ya armado del lado del cliente (ver
 * `exportarCsv` en `componentes/Tabla.tsx`). El backend agrega el BOM de
 * UTF-8 para que Excel lea bien las tildes. */
export function guardarCsv(destino: string, contenido: string): Promise<void> {
  return invoke("guardar_csv", { destino, contenido });
}
