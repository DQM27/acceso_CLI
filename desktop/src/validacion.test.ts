import { describe, expect, it } from "vitest";
import { cedulaSchema, nombreSchema, sanearSoloDigitos, sanearSoloLetras } from "./validacion";

describe("cedulaSchema", () => {
  it("acepta solo dígitos", () => {
    expect(cedulaSchema.safeParse("108470293").success).toBe(true);
  });

  it("rechaza vacío o con caracteres que no sean dígitos", () => {
    expect(cedulaSchema.safeParse("").success).toBe(false);
    expect(cedulaSchema.safeParse("1-0847-0293").success).toBe(false);
  });
});

describe("nombreSchema", () => {
  it("acepta letras (con acentos), espacios, apóstrofe y guión", () => {
    expect(nombreSchema.safeParse("José O'Neill Pérez-Ruiz").success).toBe(true);
  });

  it("rechaza vacío o con números", () => {
    expect(nombreSchema.safeParse("").success).toBe(false);
    expect(nombreSchema.safeParse("Marlon2").success).toBe(false);
  });
});

describe("sanearSoloDigitos", () => {
  it("descarta todo lo que no sea dígito", () => {
    expect(sanearSoloDigitos("1-0847-0293")).toBe("108470293");
  });
});

describe("sanearSoloLetras", () => {
  it("descarta números y símbolos, conserva acentos/espacio/apóstrofe/guión", () => {
    expect(sanearSoloLetras("José O'Neill Pérez-Ruiz 2")).toBe("José O'Neill Pérez-Ruiz ");
  });
});
