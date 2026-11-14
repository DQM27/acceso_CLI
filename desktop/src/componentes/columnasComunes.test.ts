import { describe, expect, it } from "vitest";
import type { ValueFormatterParams, ValueGetterParams } from "ag-grid-community";
import { columnaFecha, columnaGafete, columnaHora } from "./columnasComunes";

interface Fila {
  gafete_numero: number | null;
  entrada: string;
  salida: string | null;
}

const fila: Fila = { gafete_numero: null, entrada: "2026-11-13T14:05:00Z", salida: null };

function leer(columna: ReturnType<typeof columnaHora<Fila>>, datos: Fila | undefined) {
  const getter = columna.valueGetter as (p: ValueGetterParams<Fila>) => unknown;
  return getter({ data: datos } as ValueGetterParams<Fila>);
}

function formatear(columna: ReturnType<typeof columnaGafete<Fila>>, valor: unknown) {
  const formateador = columna.valueFormatter as (p: ValueFormatterParams<Fila>) => string;
  return formateador({ value: valor } as ValueFormatterParams<Fila>);
}

describe("columnas comunes", () => {
  it("gafete: el número, o S/G si entró sin gafete", () => {
    const columna = columnaGafete<Fila>();
    expect(columna.field).toBe("gafete_numero");
    expect(formatear(columna, null)).toBe("S/G");
    expect(formatear(columna, 12)).toBe("12");
  });

  it("fecha: ordena como AAAA-MM-DD y se muestra DD/MM/AAAA", () => {
    const columna = columnaFecha<Fila>("fecha_entrada", (f) => f.entrada);
    const valor = leer(columna, fila);
    expect(valor).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(formatear(columna, valor)).toMatch(/^\d{2}\/\d{2}\/\d{4}$/);
    expect(columna.type).toBe("fecha");
  });

  it("hora: HH:MM, y vacío si todavía no hay (sin salida)", () => {
    expect(leer(columnaHora<Fila>("hora_entrada", (f) => f.entrada), fila)).toMatch(/^\d{2}:\d{2}$/);
    expect(leer(columnaHora<Fila>("hora_salida", (f) => f.salida), fila)).toBe("");
    expect(leer(columnaHora<Fila>("hora_salida", (f) => f.salida), undefined)).toBe("");
  });

  it("cada pantalla conserva su colId, título y anchos (clave del layout guardado)", () => {
    const columna = columnaHora<Fila>("hora_salida", (f) => f.salida, { headerName: "Salida", flex: 0.7 });
    expect(columna).toMatchObject({ colId: "hora_salida", headerName: "Salida", flex: 0.7, minWidth: 90 });
  });
});
