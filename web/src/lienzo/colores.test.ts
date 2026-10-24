import { describe, expect, it } from "vitest";
import { OTROS } from "./calculo";
import { COLOR_OTROS, RegistroColores, colorDeRanura } from "./colores";

describe("RegistroColores", () => {
  it("asigna los colores en orden, y 'Otros' siempre en gris", () => {
    const r = new RegistroColores();
    expect(r.asignar(["A", "B", OTROS])).toEqual([colorDeRanura(0), colorDeRanura(1), COLOR_OTROS]);
  });

  it("el color sigue a la entidad: si un filtro saca a una, las demás no cambian", () => {
    const r = new RegistroColores();
    const [, colorB, colorC] = r.asignar(["A", "B", "C"]);
    expect(r.asignar(["B", "C"])).toEqual([colorB, colorC]);
  });

  it("una entidad nueva toma la primera ranura libre, sin pisar a las que están", () => {
    const r = new RegistroColores();
    r.asignar(["A", "B", "C"]);
    expect(r.asignar(["B", "D"])).toEqual([colorDeRanura(1), colorDeRanura(0)]);
  });

  it("nunca inventa un noveno color: lo que no entra va en gris", () => {
    const r = new RegistroColores();
    const nombres = Array.from({ length: 9 }, (_, i) => `E${i}`);
    const colores = r.asignar(nombres);
    expect(new Set(colores.slice(0, 8)).size).toBe(8);
    expect(colores[8]).toBe(COLOR_OTROS);
  });
});
