import { useState } from "react";
import type { RefObject } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { avisos } from "../avisos/avisos";
import { FileSpreadsheet, FileText, Sheet } from "lucide-react";
import { exportarTablaPdf, exportarTablaXlsx } from "../api/exportacion";
import type { TablaHandle } from "./Tabla";

/**
 * Excel · CSV · PDF de lo que la grilla muestra ahora (filtros, orden y
 * columnas visibles) -- los mismos tres botones de ícono que Historial de
 * contratistas, para las grillas sin exportador propio (historial de
 * proveedores y de KOF). Los datos salen de la propia grilla
 * (`datosVisibles`/`exportarCsv` de `Tabla`), así que sirve para cualquier
 * pantalla sin tocar el backend.
 */
export default function BotonesExportacion<T>({
  tablaRef,
  nombreArchivo,
  titulo,
  filtroDescripcion,
}: {
  tablaRef: RefObject<TablaHandle<T> | null>;
  /** Nombre sugerido del archivo, sin extensión (ej. "historial-proveedores"). */
  nombreArchivo: string;
  /** Título del PDF (ej. "Historial de Proveedores"). */
  titulo: string;
  /** Texto bajo el título del PDF (ej. "Filtro: Últimos 6 meses"). */
  filtroDescripcion: string;
}) {
  const [exportando, setExportando] = useState(false);

  /** Lo visible, o `null` (con el aviso ya mostrado) si no hay nada. */
  function datos() {
    const visibles = tablaRef.current?.datosVisibles();
    if (!visibles || visibles.columnas.length === 0) {
      avisos.error("No hay columnas visibles para exportar.");
      return null;
    }
    if (visibles.filas.length === 0) {
      avisos.error("No hay filas para exportar con el filtro actual.");
      return null;
    }
    return visibles;
  }

  async function exportarExcel() {
    const visibles = datos();
    if (!visibles) return;
    const destino = await save({
      title: "Exportar a Excel",
      defaultPath: `${nombreArchivo}.xlsx`,
      filters: [{ name: "Excel", extensions: ["xlsx"] }],
    });
    if (!destino) return;
    setExportando(true);
    avisos.progreso(
      exportarTablaXlsx(destino, visibles.columnas, visibles.filas).finally(() =>
        setExportando(false),
      ),
      {
        cargando: "Exportando…",
        exito: (cantidad) => `${cantidad} fila(s) exportadas.`,
      },
    );
  }

  async function exportarPdf() {
    const visibles = datos();
    if (!visibles) return;
    const destino = await save({
      title: "Exportar a PDF",
      defaultPath: `${nombreArchivo}.pdf`,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (!destino) return;
    setExportando(true);
    avisos.progreso(
      exportarTablaPdf(destino, titulo, filtroDescripcion, visibles.columnas, visibles.filas).finally(
        () => setExportando(false),
      ),
      {
        cargando: "Exportando…",
        exito: "PDF exportado.",
      },
    );
  }

  const ayuda = "respeta el filtro/orden/columnas actuales de la grilla";
  // Una sola pieza de íconos (`.segmentado`), igual que en Historial.
  return (
    <div className="segmentado" role="group" aria-label="Exportar">
      <button
        type="button"
        title={exportando ? "Exportando…" : `Exportar a Excel — ${ayuda}`}
        onClick={exportarExcel}
        disabled={exportando}
      >
        <FileSpreadsheet size={16} />
      </button>
      <button
        type="button"
        title={`Exportar a CSV — ${ayuda}`}
        onClick={() => tablaRef.current?.exportarCsv(nombreArchivo)}
        disabled={exportando}
      >
        <Sheet size={16} />
      </button>
      <button
        type="button"
        title={exportando ? "Exportando…" : `Exportar a PDF — ${ayuda}`}
        onClick={exportarPdf}
        disabled={exportando}
      >
        <FileText size={16} />
      </button>
    </div>
  );
}
