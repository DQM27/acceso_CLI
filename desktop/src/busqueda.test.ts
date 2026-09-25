import { describe, expect, it } from "vitest";
import { coincideBusqueda, plegar, textoGafete, validarNumeroGafete } from "./busqueda";

describe("plegar", () => {
  it("quita tildes y pasa a minúsculas", () => {
    expect(plegar("JOSÉ ÑANDÚ")).toBe("jose nandu");
  });
});

describe("coincideBusqueda", () => {
  it("exige todas las palabras, sin importar tildes", () => {
    expect(coincideBusqueda("jose mora", "JOSÉ PÉREZ MORA")).toBe(true);
    expect(coincideBusqueda("jose vega", "JOSÉ PÉREZ MORA")).toBe(false);
  });

  it("texto vacío nunca coincide", () => {
    expect(coincideBusqueda("  ", "lo que sea")).toBe(false);
  });
});

describe("validarNumeroGafete", () => {
  it("acepta un número positivo", () => {
    expect(validarNumeroGafete(" 7 ")).toEqual({ valido: true, numero: 7 });
  });

  it("rechaza vacío y cero", () => {
    expect(validarNumeroGafete("").valido).toBe(false);
    expect(validarNumeroGafete("0").valido).toBe(false);
  });
});

describe("textoGafete", () => {
  it("usa dos dígitos", () => {
    expect(textoGafete(7)).toBe("#07");
    expect(textoGafete(123)).toBe("#123");
  });
});
