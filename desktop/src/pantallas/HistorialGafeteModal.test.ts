import { describe, expect, it } from "vitest";
import { textoMotivo } from "./HistorialGafeteModal";

describe("textoMotivo", () => {
  it("traduce cada motivo de resolución a su texto visible", () => {
    expect(textoMotivo("Pagado")).toBe("Pagado");
    expect(textoMotivo("Aparecido")).toBe("Apareció");
    expect(textoMotivo(null)).toBe("—");
  });
});
