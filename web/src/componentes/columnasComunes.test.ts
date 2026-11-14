import { describe, expect, it } from "vitest";
import type { ColDef, ValueFormatterParams, ValueGetterParams } from "ag-grid-community";
import { columnaFecha, columnaGafete, columnaHora, textoGafete } from "./columnasComunes";

interface Fila {
  gafete_numero: number | null;
  entrada: string;
  salida: string | null;
}

const fila: Fila = { gafete_numero: null, entrada: "2026-11-13T14:05:00Z", salida: null };

function leer(columna: ColDef<Fila>, datos: Fila | undefined): unknown {
  const getter = columna.valueGetter as (p: ValueGetterParams<Fila>) => unknown;
  return getter({ data: datos } as ValueGetterParams<Fila>);
}

function formatear(columna: ColDef<Fila>, valor: unknown): string {
  const formateador = columna.valueFormatter as (p: ValueFormatterParams<Fila>) => string;
  return formateador({ value: valor } as ValueFormatterParams<Fila>);
}

describe("columnas comunes del panel", () => {
  it("gafete: el número, o S/G si entró sin gafete", () => {
    expect(textoGafete(null)).toBe("S/G");
    expect(textoGafete(12)).toBe("12");
    const columna = columnaGafete<Fila>();
    expect(columna.field).toBe("gafete_numero");
    expect(formatear(columna, null)).toBe("S/G");
  });

  it("fecha: el valor ordena como AAAA-MM-DD y se ve DD/MM/AAAA", () => {
    const columna = columnaFecha<Fila>("fecha_entrada", (f) => f.entrada);
    const valor = leer(columna, fila);
    expect(valor).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(formatear(columna, valor)).toMatch(/^\d{2}\/\d{2}\/\d{4}$/);
  });

  it("sin salida: fecha y hora muestran el texto de la clase ('Activo')", () => {
    const fecha = columnaFecha<Fila>("fecha_salida", (f) => f.salida, { sinValor: "Activo" });
    const hora = columnaHora<Fila>("hora_salida", (f) => f.salida, { sinValor: "Activo" });
    expect(leer(fecha, fila)).toBe("Activo");
    expect(formatear(fecha, "Activo")).toBe("Activo");
    expect(leer(hora, fila)).toBe("Activo");
  });

  it("hora: HH:MM, sin filtro propio, y vacía si no hay dato", () => {
    const columna = columnaHora<Fila>("hora_entrada", (f) => f.entrada);
    expect(leer(columna, fila)).toMatch(/^\d{2}:\d{2}$/);
    expect(leer(columna, undefined)).toBe("");
    expect(columna.filter).toBe(false);
  });

  it("cada pantalla conserva su colId, título y anchos", () => {
    const columna = columnaFecha<Fila>("fecha_ingreso", (f) => f.entrada, { headerName: "Fecha ingreso", flex: 1.4 });
    expect(columna).toMatchObject({ colId: "fecha_ingreso", headerName: "Fecha ingreso", flex: 1.4, type: "fecha" });
  });
});
