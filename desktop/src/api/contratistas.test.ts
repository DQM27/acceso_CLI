import { describe, expect, it } from "vitest";
import { textoTipoIngreso } from "./contratistas";

describe("textoTipoIngreso", () => {
  it("en mayusculas y con las palabras separadas", () => {
    expect(textoTipoIngreso("Praind")).toBe("PRAIND");
    expect(textoTipoIngreso("InHouse")).toBe("IN HOUSE");
    expect(textoTipoIngreso("PorCorreo")).toBe("POR CORREO");
    expect(textoTipoIngreso("Swat")).toBe("SWAT");
    expect(textoTipoIngreso(null)).toBe("\u2014");
  });
});
