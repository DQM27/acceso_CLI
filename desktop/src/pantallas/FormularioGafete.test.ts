import { describe, expect, it } from "vitest";
import { esquema, resumenCreacion } from "./FormularioGafete";

const base = { modo: "individual", tipo: "contratista", numero: "", desde: "", hasta: "" } as const;

describe("resumenCreacion", () => {
  it("individual: nombra el gafete y el tipo", () => {
    expect(resumenCreacion({ ...base, numero: "7" })).toEqual({
      frase: "Se creará el gafete #07 de contratista.",
      boton: "Crear gafete",
    });
  });

  it("rango: cuenta los gafetes y los extremos", () => {
    expect(
      resumenCreacion({ ...base, modo: "rango", tipo: "provisional_kof", desde: "1", hasta: "20" }),
    ).toEqual({
      frase: "Se crearán 20 gafetes de provisional KOF, del #01 al #20.",
      boton: "Crear 20 gafetes",
    });
  });

  it("rango de uno: en singular", () => {
    expect(resumenCreacion({ ...base, modo: "rango", desde: "5", hasta: "5" }).boton).toBe(
      "Crear gafete",
    );
  });

  it("sin números suficientes no hay frase", () => {
    expect(resumenCreacion(base).frase).toBeNull();
    expect(resumenCreacion({ ...base, modo: "rango", desde: "9", hasta: "3" }).frase).toBeNull();
  });
});

describe("esquema de FormularioGafete", () => {
  it("individual: número vacío o no numérico no pasa", () => {
    expect(
      esquema.safeParse({ modo: "individual", tipo: "contratista", numero: "", desde: "", hasta: "" })
        .success,
    ).toBe(false);
    expect(
      esquema.safeParse({ modo: "individual", tipo: "contratista", numero: "abc", desde: "", hasta: "" })
        .success,
    ).toBe(false);
    expect(
      esquema.safeParse({ modo: "individual", tipo: "contratista", numero: "0", desde: "", hasta: "" })
        .success,
    ).toBe(false);
  });

  it("individual: número válido pasa", () => {
    expect(
      esquema.safeParse({ modo: "individual", tipo: "contratista", numero: "12", desde: "", hasta: "" })
        .success,
    ).toBe(true);
  });

  it("rango: hasta menor a desde no pasa", () => {
    expect(
      esquema.safeParse({ modo: "rango", tipo: "visita", numero: "", desde: "9", hasta: "3" }).success,
    ).toBe(false);
  });

  it("rango: más de 200 gafetes de una vez no pasa", () => {
    expect(
      esquema.safeParse({ modo: "rango", tipo: "visita", numero: "", desde: "1", hasta: "300" })
        .success,
    ).toBe(false);
  });

  it("rango válido pasa", () => {
    expect(
      esquema.safeParse({ modo: "rango", tipo: "visita", numero: "", desde: "1", hasta: "25" }).success,
    ).toBe(true);
  });

  it("tipo invalido no pasa", () => {
    expect(
      esquema.safeParse({ modo: "individual", tipo: "inexistente", numero: "12", desde: "", hasta: "" })
        .success,
    ).toBe(false);
  });

  it("tipo provisional_kof pasa", () => {
    expect(
      esquema.safeParse({
        modo: "individual",
        tipo: "provisional_kof",
        numero: "12",
        desde: "",
        hasta: "",
      }).success,
    ).toBe(true);
  });

  it("tipo proveedor pasa", () => {
    expect(
      esquema.safeParse({
        modo: "individual",
        tipo: "proveedor",
        numero: "12",
        desde: "",
        hasta: "",
      }).success,
    ).toBe(true);
  });
});
