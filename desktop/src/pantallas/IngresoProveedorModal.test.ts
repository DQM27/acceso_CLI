import { describe, expect, it } from "vitest";
import { datosNuevoDesdeBusqueda, proveedoresConocidos } from "./IngresoProveedorModal";

const ingreso = (cedula: string, nombre: string, empresa: string | null, fecha: string) => ({
  cedula,
  nombre,
  empresa_nombre: empresa,
  fecha_hora_ingreso: fecha,
});

describe("proveedoresConocidos", () => {
  it("deja uno por cédula, con los datos de su ingreso más reciente", () => {
    const conocidos = proveedoresConocidos([
      ingreso("101", "ANA VIEJO", "EMPRESA A", "2026-09-01T10:00:00Z"),
      ingreso("202", "LUIS", "EMPRESA B", "2026-09-10T10:00:00Z"),
      ingreso("101", "ANA NUEVO", "EMPRESA C", "2026-09-20T10:00:00Z"),
    ]);
    expect(conocidos).toEqual([
      { cedula: "101", nombre: "ANA NUEVO", empresa_nombre: "EMPRESA C" },
      { cedula: "202", nombre: "LUIS", empresa_nombre: "EMPRESA B" },
    ]);
  });

  it("ignora filas sin cédula", () => {
    expect(proveedoresConocidos([ingreso("  ", "X", null, "2026-09-01T10:00:00Z")])).toEqual([]);
  });
});

describe("datosNuevoDesdeBusqueda", () => {
  it("números van a la cédula", () => {
    expect(datosNuevoDesdeBusqueda(" 1-0847-0293 ")).toEqual({ cedula: "1-0847-0293", nombre: "" });
  });

  it("texto va al nombre, en mayúsculas", () => {
    expect(datosNuevoDesdeBusqueda("juan pérez")).toEqual({ cedula: "", nombre: "JUAN PÉREZ" });
  });
});
