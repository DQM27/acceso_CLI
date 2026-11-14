import { describe, expect, it } from "vitest";
import {
  filtrosVisiblesIniciales,
  ocultasIniciales,
  plegar,
  plegarConCache,
  tamanoCachePlegado,
  TOPE_CACHE_PLEGADO,
} from "./Tabla.logica";

describe("plegar / plegarConCache", () => {
  it("quita tildes y pasa a mayúsculas", () => {
    expect(plegar("Sánchez Ureña")).toBe("SANCHEZ URENA");
  });

  it("la versión con caché devuelve lo mismo y recuerda cada texto una vez", () => {
    const antes = tamanoCachePlegado();
    expect(plegarConCache("José Pérez")).toBe("JOSE PEREZ");
    plegarConCache("José Pérez");
    expect(tamanoCachePlegado()).toBe(antes + 1);
  });

  it("no crece sin límite", () => {
    for (let i = 0; i <= TOPE_CACHE_PLEGADO; i++) plegarConCache(`fila ${i}`);
    expect(tamanoCachePlegado()).toBeLessThanOrEqual(TOPE_CACHE_PLEGADO);
  });
});

describe("filtrosVisiblesIniciales", () => {
  it("sin nada guardado, la fila de filtros arranca apagada", () => {
    expect(filtrosVisiblesIniciales(null)).toBe(false);
  });

  it("un layout viejo con filtrosVisibles en true sin elección explícita arranca apagado", () => {
    expect(filtrosVisiblesIniciales({ ocultas: [], filtrosVisibles: true })).toBe(false);
  });

  it("respeta lo que la persona eligió a mano", () => {
    expect(filtrosVisiblesIniciales({ ocultas: [], filtrosVisibles: true, filtrosElegidos: true })).toBe(true);
  });
});

describe("ocultasIniciales", () => {
  const columnas = [{ field: "nombre" }, { field: "usuario_entrada_nombre", hide: true }, { colId: "tiempo" }];

  it("sin layout guardado oculta las columnas marcadas hide", () => {
    expect(ocultasIniciales(null, columnas)).toEqual(["usuario_entrada_nombre"]);
  });

  it("con layout guardado manda lo que eligió la persona", () => {
    expect(ocultasIniciales({ ocultas: ["nombre"] }, columnas)).toEqual(["nombre"]);
  });
});
