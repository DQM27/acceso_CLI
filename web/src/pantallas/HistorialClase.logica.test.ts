import { describe, expect, it } from "vitest";
import type { MovimientoClase } from "../api/historialClases";
import { columnasClase, definicionesExportClase } from "./HistorialClase.logica";

const movimiento: MovimientoClase = {
  id: "k1",
  sitio_id: "s1",
  sitio_nombre: "Brisas",
  identificacion: "EMP-1",
  nombre: "ENCARGADO",
  gafete_numero: 3,
  hora_entrada: "2026-10-20T14:30:00Z",
  hora_salida: null,
  usuario_entrada_nombre: "OP",
  usuario_salida_nombre: null,
  dispositivo_entrada_tipo: "pc",
};

const etiquetas = (clase: Parameters<typeof columnasClase>[0]) =>
  columnasClase(clase).map((columna) => columna.headerName);

describe("columnas de cada clase del historial", () => {
  it("proveedor con empresa y placa; correo con motivo; KOF sin ninguna de las dos", () => {
    expect(etiquetas("proveedores")).toContain("Empresa");
    expect(etiquetas("proveedores")).toContain("Placa");
    expect(etiquetas("correo")).toContain("Motivo");
    expect(etiquetas("kof")).not.toContain("Placa");
    expect(etiquetas("kof")).toEqual(expect.arrayContaining(["Código empleado", "Encargado", "Entrega", "Devolución", "Entregó", "Recibió"]));
  });

  it("la grilla y la exportación usan los mismos colId (se exporta lo visible)", () => {
    for (const clase of ["proveedores", "correo", "kof"] as const) {
      const deGrilla = columnasClase(clase).map((columna) => columna.colId ?? columna.field);
      const deExport = definicionesExportClase(clase).map((definicion) => definicion.colId);
      expect(deExport).toEqual(deGrilla);
    }
  });

  it("sin devolución, KOF dice Prestado; sin gafete, S/G", () => {
    const valor = (colId: string, fila: MovimientoClase) =>
      definicionesExportClase("kof").find((definicion) => definicion.colId === colId)?.valor(fila);
    expect(valor("hora_salida", movimiento)).toBe("Prestado");
    expect(valor("fecha_salida", movimiento)).toBe("Prestado");
    expect(valor("gafete_numero", { ...movimiento, gafete_numero: null })).toBe("S/G");
  });
});
