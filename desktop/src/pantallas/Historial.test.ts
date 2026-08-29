import { describe, expect, it } from "vitest";
import { textoMedio } from "./Historial";

// fechaLocalYMD/textoHora/textoFechaDDMMYYYY se probaron en
// src/tiempo.test.ts — Historial las importa de ahí, no las define más.

describe("textoMedio", () => {
  it("Vehiculo -> Vehículo, cualquier otro -> Caminando", () => {
    expect(textoMedio("Vehiculo")).toBe("Vehículo");
    expect(textoMedio("Caminando")).toBe("Caminando");
  });
});
