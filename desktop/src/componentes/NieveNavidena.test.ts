import { describe, expect, it } from "vitest";
import {
  cantidadCopos,
  crearCopos,
  esFondoOscuro,
  posicionCopo,
} from "./NieveNavidena.logica";

describe("NieveNavidena.logica", () => {
  it("crea siempre los mismos copos con la misma semilla", () => {
    expect(crearCopos(10)).toEqual(crearCopos(10));
    expect(crearCopos(10)).toHaveLength(10);
  });

  it("la cantidad sigue el tamaño de la ventana, con piso y techo", () => {
    expect(cantidadCopos(1920, 1080)).toBe(130);
    expect(cantidadCopos(200, 200)).toBe(20);
    expect(cantidadCopos(7680, 4320)).toBe(140);
  });

  it("cada copo cae dentro de la ventana y vuelve a empezar arriba", () => {
    const [copo] = crearCopos(1);
    for (let s = 0; s < 120; s += 0.5) {
      const { x, y } = posicionCopo(copo, s, 1000, 800);
      expect(y).toBeGreaterThanOrEqual(-6);
      expect(y).toBeLessThanOrEqual(806);
      expect(x).toBeGreaterThan(-50);
      expect(x).toBeLessThan(1050);
    }
  });

  it("distingue fondo oscuro de claro", () => {
    expect(esFondoOscuro("rgb(26, 27, 38)")).toBe(true);
    expect(esFondoOscuro("rgb(240, 242, 245)")).toBe(false);
    expect(esFondoOscuro("")).toBe(true);
  });
});
