import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { ColDef } from "ag-grid-community";
import {
  claveAlmacenamiento,
  clicEnControlInteractivo,
  compararFechaYMD,
  identidad,
  leerEstadoGuardado,
  filtrosVisiblesIniciales,
  ocultasIniciales,
  plegar,
  plegarConCache,
  tamanoCachePlegado,
  textoTooltip,
  TOPE_CACHE_PLEGADO,
} from "./Tabla.logica";
import type { ITooltipParams } from "ag-grid-community";
import type { EstadoGuardado } from "./Tabla.logica";

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

describe("textoTooltip", () => {
  const params = (valueFormatted: unknown, value: unknown) =>
    ({ valueFormatted, value }) as unknown as ITooltipParams;

  it("prefiere el valor formateado (ej. S/G)", () => {
    expect(textoTooltip(params("S/G", null))).toBe("S/G");
  });

  it("cae al valor crudo de texto o numero", () => {
    expect(textoTooltip(params(null, "KAREN DE LOS ANGELES"))).toBe("KAREN DE LOS ANGELES");
    expect(textoTooltip(params(undefined, 42))).toBe("42");
  });

  it("sin tooltip para booleanos, objetos o vacios", () => {
    expect(textoTooltip(params(undefined, true))).toBeUndefined();
    expect(textoTooltip(params(undefined, { a: 1 }))).toBeUndefined();
    expect(textoTooltip(params("", ""))).toBeUndefined();
  });
});

describe("clicEnControlInteractivo", () => {
  it("un clic dentro de un boton no cuenta como clic de fila", () => {
    const boton = document.createElement("button");
    const icono = document.createElement("span");
    boton.appendChild(icono);
    expect(clicEnControlInteractivo(boton)).toBe(true);
    expect(clicEnControlInteractivo(icono)).toBe(true);
  });

  it("un clic en texto de la celda si cuenta", () => {
    const celda = document.createElement("div");
    expect(clicEnControlInteractivo(celda)).toBe(false);
    expect(clicEnControlInteractivo(null)).toBe(false);
  });
});

describe("compararFechaYMD", () => {
  const filtro = new Date(2026, 8, 23);

  it("compara el dia de la celda contra el del filtro", () => {
    expect(compararFechaYMD(filtro, "2026-09-23")).toBe(0);
    expect(compararFechaYMD(filtro, "2026-09-22")).toBeLessThan(0);
    expect(compararFechaYMD(filtro, "2026-09-24")).toBeGreaterThan(0);
  });

  it("un valor que no es fecha (ej. Activo) queda antes de cualquier fecha", () => {
    expect(compararFechaYMD(filtro, "Activo")).toBeLessThan(0);
    expect(compararFechaYMD(filtro, null)).toBeLessThan(0);
  });
});

describe("plegar", () => {
  it("quita tildes y pasa a mayúsculas, igual que PLEGAR de SQLite", () => {
    expect(plegar("Sánchez Ureña")).toBe("SANCHEZ URENA");
    expect(plegar("ÁLVAREZ")).toBe("ALVAREZ");
  });
});

describe("plegarConCache", () => {
  it("devuelve lo mismo que plegar", () => {
    expect(plegarConCache("José Pablo Jiménez")).toBe(plegar("José Pablo Jiménez"));
  });

  it("recuerda cada texto una sola vez", () => {
    const antes = tamanoCachePlegado();
    plegarConCache("texto único de prueba á");
    plegarConCache("texto único de prueba á");
    expect(tamanoCachePlegado()).toBe(antes + 1);
  });

  it("al llegar al tope se vacía en vez de crecer sin límite", () => {
    for (let i = 0; i <= TOPE_CACHE_PLEGADO; i++) plegarConCache(`fila ${i}`);
    expect(tamanoCachePlegado()).toBeLessThanOrEqual(TOPE_CACHE_PLEGADO);
    expect(plegarConCache("Peña")).toBe("PENA");
  });
});

describe("filtrosVisiblesIniciales", () => {
  it("sin nada guardado, la fila de filtros arranca apagada", () => {
    expect(filtrosVisiblesIniciales(null)).toBe(false);
  });

  it("un layout viejo con filtrosVisibles en true pero sin elección explícita arranca apagado", () => {
    expect(filtrosVisiblesIniciales({ ocultas: [], columnas: [], filtrosVisibles: true })).toBe(false);
  });

  it("respeta lo que la persona eligió a mano", () => {
    const base = { ocultas: [], columnas: [], filtrosElegidos: true };
    expect(filtrosVisiblesIniciales({ ...base, filtrosVisibles: true })).toBe(true);
    expect(filtrosVisiblesIniciales({ ...base, filtrosVisibles: false })).toBe(false);
  });
});

describe("ocultasIniciales", () => {
  const columnas = [
    { field: "nombre" },
    { field: "usuario_ingreso_nombre", hide: true },
    { colId: "tiempo", hide: false },
  ] as ColDef<unknown>[];

  it("sin layout guardado oculta las columnas marcadas hide en el código", () => {
    expect(ocultasIniciales(null, columnas)).toEqual(["usuario_ingreso_nombre"]);
  });

  it("con layout guardado manda lo que eligió la persona", () => {
    expect(ocultasIniciales({ ocultas: ["nombre"], columnas: [] }, columnas)).toEqual(["nombre"]);
  });
});
