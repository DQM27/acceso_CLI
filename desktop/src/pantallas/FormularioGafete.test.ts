import { describe, expect, it } from "vitest";
import { esquema } from "./FormularioGafete";

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
      esquema.safeParse({ modo: "individual", tipo: "proveedor", numero: "12", desde: "", hasta: "" })
        .success,
    ).toBe(false);
  });
});
