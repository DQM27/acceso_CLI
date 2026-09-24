import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { TablaHandle } from "./Tabla";

const save = vi.fn();
const exportarTablaXlsx = vi.fn(() => Promise.resolve(2));
const exportarTablaPdf = vi.fn(() => Promise.resolve());
const toastError = vi.fn();

vi.mock("@tauri-apps/plugin-dialog", () => ({ save: (...args: unknown[]) => save(...args) }));
vi.mock("../api/exportacion", () => ({
  exportarTablaXlsx: (...args: unknown[]) => exportarTablaXlsx(...(args as [])),
  exportarTablaPdf: (...args: unknown[]) => exportarTablaPdf(...(args as [])),
}));
vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), {
    error: (...args: unknown[]) => toastError(...args),
    promise: vi.fn(),
  }),
}));

import BotonesExportacion from "./BotonesExportacion";

const columnas = [
  { titulo: "NOMBRE", izquierda: true },
  { titulo: "GAFETE", izquierda: false },
];

function referencia(filas: string[][]) {
  const handle = {
    datosVisibles: () => ({ columnas, filas }),
    exportarCsv: vi.fn(() => Promise.resolve()),
  } as unknown as TablaHandle<unknown>;
  return { current: handle };
}

function montar(filas: string[][]) {
  const tablaRef = referencia(filas);
  render(
    <BotonesExportacion
      tablaRef={tablaRef}
      nombreArchivo="historial-proveedores"
      titulo="Historial de Proveedores"
      filtroDescripcion="Filtro: Hoy"
    />,
  );
  return tablaRef;
}

describe("BotonesExportacion", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("Excel manda a exportar exactamente lo que muestra la grilla", async () => {
    save.mockResolvedValue("C:/x/historial-proveedores.xlsx");
    montar([["Ana Solano", "S/G"]]);

    fireEvent.click(screen.getByTitle(/Exportar a Excel/));

    await waitFor(() =>
      expect(exportarTablaXlsx).toHaveBeenCalledWith("C:/x/historial-proveedores.xlsx", columnas, [
        ["Ana Solano", "S/G"],
      ]),
    );
  });

  it("PDF lleva el título y la descripción del filtro", async () => {
    save.mockResolvedValue("C:/x/historial-proveedores.pdf");
    montar([["Ana Solano", "S/G"]]);

    fireEvent.click(screen.getByTitle(/Exportar a PDF/));

    await waitFor(() =>
      expect(exportarTablaPdf).toHaveBeenCalledWith(
        "C:/x/historial-proveedores.pdf",
        "Historial de Proveedores",
        "Filtro: Hoy",
        columnas,
        [["Ana Solano", "S/G"]],
      ),
    );
  });

  it("sin filas avisa y ni siquiera abre el diálogo de guardar", async () => {
    montar([]);

    fireEvent.click(screen.getByTitle(/Exportar a Excel/));

    await waitFor(() => expect(toastError).toHaveBeenCalled());
    expect(save).not.toHaveBeenCalled();
    expect(exportarTablaXlsx).not.toHaveBeenCalled();
  });

  it("CSV usa el exportador de la propia grilla con el nombre del archivo", () => {
    const tablaRef = montar([["Ana Solano", "S/G"]]);

    fireEvent.click(screen.getByTitle(/Exportar a CSV/));

    expect(tablaRef.current.exportarCsv).toHaveBeenCalledWith("historial-proveedores");
  });
});
