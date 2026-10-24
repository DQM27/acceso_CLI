import { CATALOGO, type Origen } from "./catalogo";

/** Dónde vive el sandbox del panel "libre" y de Python (carpeta `sandbox-lienzo/`, Worker `lienzo-sandbox`). */
export const URL_SANDBOX: string = import.meta.env.VITE_LIENZO_SANDBOX_URL || "https://lienzo.megabrisas.com/";

export interface ColumnaParaSandbox {
  nombre: string;
  tipo: string;
  etiqueta: string;
}

/** Las columnas de las filas que se mandan al sandbox: las del catálogo o las de la consulta, con su etiqueta. */
export function columnasParaSandbox(origen: Origen): ColumnaParaSandbox[] {
  if (typeof origen !== "string") return origen.columnas.map((c) => ({ nombre: c.nombre, tipo: c.tipo, etiqueta: c.etiqueta ?? c.nombre }));
  return Object.entries(CATALOGO[origen].campos).map(([nombre, def]) => ({ nombre, tipo: def.tipo, etiqueta: def.etiqueta }));
}
