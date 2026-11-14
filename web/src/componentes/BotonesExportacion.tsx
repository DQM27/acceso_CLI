import { FileSpreadsheet, FileText, Sheet } from "lucide-react";

/**
 * Excel · CSV · PDF en una sola pieza de íconos (`.segmentado`), como en el
 * escritorio. Los tres traen TODO el resultado del filtro actual (no sólo la
 * página en pantalla), con las columnas visibles y el orden vigente; cada
 * pantalla decide cómo (`alExportar`).
 *
 * Estaba copiado en HistorialContratistas e HistorialClase (auditoría
 * 2026-11-14).
 */
export default function BotonesExportacion({
  exportando,
  alExportarExcel,
  alExportarCsv,
  alExportarPdf,
}: {
  exportando: boolean;
  alExportarExcel: () => void;
  alExportarCsv: () => void;
  alExportarPdf: () => void;
}) {
  const titulo = (formato: string) =>
    exportando ? "Exportando…" : `Exportar a ${formato} — todo el resultado del filtro actual, con las columnas visibles`;

  return (
    <div className="segmentado" role="group" aria-label="Exportar">
      <button
        type="button"
        title={titulo("Excel")}
        aria-label="Exportar a Excel"
        onClick={alExportarExcel}
        disabled={exportando}
      >
        <FileSpreadsheet size={16} />
      </button>
      <button
        type="button"
        title={titulo("CSV")}
        aria-label="Exportar a CSV"
        onClick={alExportarCsv}
        disabled={exportando}
      >
        <Sheet size={16} />
      </button>
      <button
        type="button"
        title={titulo("PDF")}
        aria-label="Exportar a PDF"
        onClick={alExportarPdf}
        disabled={exportando}
      >
        <FileText size={16} />
      </button>
    </div>
  );
}
