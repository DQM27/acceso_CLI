import { describe, expect, it } from "vitest";
import { textoIdentidadEquipo, textoUnidadOperativa } from "./IdentidadEquipo.logica";

describe("textoIdentidadEquipo", () => {
  it("muestra la unidad y la etiqueta", () => {
    expect(textoIdentidadEquipo({ unidad: "Planta Cartago", etiqueta: "PC portería norte" })).toBe(
      "Unidad: Planta Cartago · PC portería norte",
    );
  });

  it("muestra lo que haya si falta una parte", () => {
    expect(textoIdentidadEquipo({ unidad: "Planta Cartago", etiqueta: null })).toBe(
      "Unidad: Planta Cartago",
    );
    expect(textoIdentidadEquipo({ unidad: null, etiqueta: "PC portería norte" })).toBe(
      "Equipo: PC portería norte",
    );
  });

  it("no muestra nada si todavía no llegó", () => {
    expect(textoIdentidadEquipo(null)).toBeNull();
    expect(textoIdentidadEquipo({ unidad: null, etiqueta: null })).toBeNull();
    expect(textoIdentidadEquipo({ unidad: "  ", etiqueta: "" })).toBeNull();
  });
});

describe("textoUnidadOperativa", () => {
  it("muestra sólo la unidad, en mayúsculas", () => {
    expect(textoUnidadOperativa({ unidad: "Brisas", etiqueta: "PC portería norte" })).toBe("BRISAS");
    expect(textoUnidadOperativa({ unidad: " Belén Cartago ", etiqueta: null })).toBe("BELÉN CARTAGO");
  });

  it("no muestra nada sin unidad, aunque haya etiqueta", () => {
    expect(textoUnidadOperativa(null)).toBeNull();
    expect(textoUnidadOperativa({ unidad: null, etiqueta: "PC portería norte" })).toBeNull();
    expect(textoUnidadOperativa({ unidad: "  ", etiqueta: null })).toBeNull();
  });
});
