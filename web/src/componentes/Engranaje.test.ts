import { describe, expect, it } from "vitest";
import { trazoEngranaje } from "./Engranaje.logica";

describe("trazoEngranaje", () => {
  it("arranca con un movimiento y cierra el trazo", () => {
    const trazo = trazoEngranaje(8, 47, 37);
    expect(trazo.startsWith("M")).toBe(true);
    expect(trazo.endsWith("Z")).toBe(true);
  });

  it("tiene un arco entre cada par de dientes", () => {
    for (const dientes of [6, 8, 12]) {
      expect(trazoEngranaje(dientes, 47, 37).split("A").length - 1).toBe(dientes);
    }
  });

  it("no se sale del lienzo de 100 x 100", () => {
    const numeros = (trazoEngranaje(8, 47, 37).match(/-?\d+\.\d+/g) ?? []).map(Number);
    expect(numeros.length).toBeGreaterThan(0);
    expect(Math.min(...numeros)).toBeGreaterThanOrEqual(0);
    expect(Math.max(...numeros)).toBeLessThanOrEqual(100);
  });
});
