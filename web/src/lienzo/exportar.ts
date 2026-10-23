import {
  descargarArchivo,
  descargarExcel,
  generarCsv,
  generarHtmlTabla,
  imprimirHtml,
  type DefinicionColumnaExport,
} from "../exportacion";
import { CATALOGO, type FormatoFecha, type Origen } from "./catalogo";
import { formatearFecha, type Fila } from "./calculo";
import { nombreDeArchivo } from "./imagen";

/**
 * Exportar los datos de una vista del lienzo (las filas de su fuente, con sus
 * filtros) a CSV, Excel o PDF, con los mismos formatos que el resto del panel
 * (`exportacion.ts`): lo que se baja del lienzo se ve igual que lo del Historial.
 */

export type FormatoExportacion = "csv" | "excel" | "pdf";

/** Las columnas de la vista: las del catálogo o las que declara su consulta, con sus etiquetas. */
export function columnasParaExportar(origen: Origen, formatoFecha?: FormatoFecha): DefinicionColumnaExport<Fila>[] {
  const columnas =
    typeof origen === "string"
      ? Object.entries(CATALOGO[origen].campos).map(([nombre, def]) => ({ nombre, tipo: def.tipo as string, etiqueta: def.etiqueta }))
      : origen.columnas.map((c) => ({ nombre: c.nombre, tipo: c.tipo as string, etiqueta: c.etiqueta ?? c.nombre }));
  return columnas.map((c) => ({
    colId: c.nombre,
    etiqueta: c.etiqueta,
    izquierda: c.tipo === "texto" || c.tipo === "enumeracion",
    valor: (fila: Fila) => {
      const v = fila[c.nombre];
      if (v === null || v === undefined) return "";
      if (typeof v === "boolean") return v ? "Sí" : "No";
      if (c.tipo === "fecha" && typeof v === "string") return formatearFecha(v.slice(0, 10), formatoFecha);
      return String(v);
    },
  }));
}

export async function exportarFilas(
  formato: FormatoExportacion,
  { titulo, origen, filas, formatoFecha, generadoPor, filtros }: {
    titulo: string;
    origen: Origen;
    filas: Fila[];
    formatoFecha?: FormatoFecha;
    generadoPor: string;
    /** Los filtros de la vista en palabras, para el PDF. */
    filtros: string;
  },
): Promise<void> {
  const columnas = columnasParaExportar(origen, formatoFecha);
  if (formato === "csv") {
    descargarArchivo(nombreDeArchivo(titulo, "csv"), generarCsv(filas, columnas), "text/csv;charset=utf-8");
  } else if (formato === "excel") {
    await descargarExcel(filas, columnas, { hoja: titulo.slice(0, 31) || "Lienzo", archivo: nombreDeArchivo(titulo, "xlsx") });
  } else if (!imprimirHtml(generarHtmlTabla(filas, columnas, { titulo, generadoPor, filtro: filtros || "Sin filtros" }))) {
    throw new Error("No se pudo preparar el PDF.");
  }
}
