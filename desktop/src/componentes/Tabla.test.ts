import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { ColDef } from "ag-grid-community";
import { claveAlmacenamiento, identidad, leerEstadoGuardado } from "./Tabla";
import type { EstadoGuardado } from "./Tabla";

// `ColDef<unknown>` infiere `field` como `keyof unknown` (efectivamente
// `never`) — en los tests no hay un tipo de fila real, así que se castea a
// propósito en vez de inventar uno sólo para esto.
function col(props: { colId?: string; field?: string }): ColDef<unknown> {
  return props as ColDef<unknown>;
}

describe("claveAlmacenamiento", () => {
  it("namespacea por id e incluye la versión del layout", () => {
    expect(claveAlmacenamiento("activos")).toBe("tabla:activos:v2");
    expect(claveAlmacenamiento("contratistas")).toBe("tabla:contratistas:v2");
  });
});

describe("identidad", () => {
  it("usa colId cuando está explícito, aunque haya field", () => {
    expect(identidad(col({ colId: "hora_ingreso", field: "fecha_hora_ingreso" }))).toBe(
      "hora_ingreso",
    );
  });

  it("cae a field cuando no hay colId", () => {
    expect(identidad(col({ field: "contratista_nombre" }))).toBe("contratista_nombre");
  });

  it("sin colId ni field, no hay identidad", () => {
    expect(identidad(col({}))).toBeUndefined();
  });
});

describe("leerEstadoGuardado", () => {
  const ESTADO: EstadoGuardado = {
    ocultas: ["gafete_numero"],
    columnas: [],
  };

  beforeEach(() => {
    localStorage.clear();
  });
  afterEach(() => {
    localStorage.clear();
  });

  it("sin id, no intenta leer nada", () => {
    expect(leerEstadoGuardado(undefined)).toBeNull();
  });

  it("sin nada guardado, null", () => {
    expect(leerEstadoGuardado("activos")).toBeNull();
  });

  it("devuelve el estado guardado bajo la clave versionada", () => {
    localStorage.setItem(claveAlmacenamiento("activos"), JSON.stringify(ESTADO));
    expect(leerEstadoGuardado("activos")).toEqual(ESTADO);
  });

  it("un layout de otra grilla (id distinto) no se mezcla", () => {
    localStorage.setItem(claveAlmacenamiento("activos"), JSON.stringify(ESTADO));
    expect(leerEstadoGuardado("contratistas")).toBeNull();
  });

  it("JSON corrupto no tira, devuelve null", () => {
    localStorage.setItem(claveAlmacenamiento("activos"), "{esto no es json");
    expect(leerEstadoGuardado("activos")).toBeNull();
  });
});
