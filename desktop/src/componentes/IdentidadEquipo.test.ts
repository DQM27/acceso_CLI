import { describe, expect, it } from "vitest";
import { textoIdentidadEquipo } from "./IdentidadEquipo.logica";

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
