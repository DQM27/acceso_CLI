import { describe, expect, it } from "vitest";
import { avisoLectura, avisoPrevio, parsearNumero, textoNumero } from "./ConteoGafetesModal.logica";

describe("parsearNumero", () => {
  it("acepta números positivos y rechaza lo demás", () => {
    expect(parsearNumero(" 7 ")).toBe(7);
    expect(parsearNumero("07")).toBe(7);
    expect(parsearNumero("0")).toBeNull();
    expect(parsearNumero("-3")).toBeNull();
    expect(parsearNumero("7a")).toBeNull();
    expect(parsearNumero("")).toBeNull();
  });
});

describe("textoNumero", () => {
  it("muestra dos dígitos como mínimo", () => {
    expect(textoNumero(7)).toBe("07");
    expect(textoNumero(125)).toBe("125");
  });
});

describe("avisoPrevio", () => {
  it("rechaza lo que no es número y lo repetido", () => {
    expect(avisoPrevio("abc", [])).toMatchObject({ nivel: "error", anotar: false });
    expect(avisoPrevio("7", [3, 7])).toEqual({
      nivel: "error",
      texto: "El 07 ya lo había colocado.",
      anotar: false,
    });
  });

  it("deja pasar un número nuevo para preguntarle al catálogo", () => {
    expect(avisoPrevio("12", [3, 7])).toBeNull();
  });
});

describe("avisoLectura", () => {
  it("anota lo que está en la mesa y rechaza lo que no existe", () => {
    expect(avisoLectura(3, { tipo: "esperado" })).toMatchObject({ nivel: "bien", anotar: true });
    expect(avisoLectura(99, { tipo: "no_existe" })).toEqual({
      nivel: "error",
      texto: "El 99 no existe en el catálogo.",
      anotar: false,
    });
    expect(avisoLectura(5, { tipo: "perdido" })).toMatchObject({ nivel: "atencion", anotar: true });
    expect(avisoLectura(4, { tipo: "en_uso", nombre: "Juan Pérez" })).toMatchObject({
      nivel: "atencion",
      anotar: true,
    });
    expect(avisoLectura(4, { tipo: "en_uso", nombre: "Juan Pérez" }).texto).toContain("Juan Pérez");
  });
});
