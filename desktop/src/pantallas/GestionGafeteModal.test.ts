import { describe, expect, it } from "vitest";
import { textoEstado } from "./GestionGafeteModal";

describe("textoEstado", () => {
  it("traduce cada estado a su texto visible", () => {
    expect(textoEstado("Disponible")).toBe("Disponible");
    expect(textoEstado("Perdido")).toBe("Perdido");
    expect(textoEstado("DeBaja")).toBe("De baja");
  });
});
