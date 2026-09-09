import { describe, expect, it } from "vitest";
import { textoUnidadesOperativas } from "./SelectorUnidadesOperativas";

describe("textoUnidadesOperativas", () => {
  it("sin unidades cargadas todavía", () => {
    expect(textoUnidadesOperativas(0, 0)).toBe("Unidades operativas");
  });

  it("nada excluido -- todas", () => {
    expect(textoUnidadesOperativas(3, 0)).toBe("Todas las unidades");
  });

  it("algunas excluidas -- cuenta incluidas de total", () => {
    expect(textoUnidadesOperativas(3, 1)).toBe("2 de 3 unidades");
  });

  it("todas excluidas -- ninguna", () => {
    expect(textoUnidadesOperativas(3, 3)).toBe("Ninguna unidad");
  });
});
