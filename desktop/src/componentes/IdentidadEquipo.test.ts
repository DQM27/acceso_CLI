import { describe, expect, it } from "vitest";
import { textoUnidadOperativa } from "./IdentidadEquipo.logica";

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
