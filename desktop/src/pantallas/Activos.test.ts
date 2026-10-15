import { describe, expect, it } from "vitest";
import {
  contarMasDeDoceHoras,
  contarPorGafete,
  filtrarPorGafete,
  masDeDoceHoras,
  textoAvisoPermanencia,
} from "./Activos.logica";

const filas = [
  { id: 1, gafete_numero: 10 },
  { id: 2, gafete_numero: null },
  { id: 3, gafete_numero: 0 },
  { id: 4, gafete_numero: null },
];

describe("contarPorGafete", () => {
  it("cuenta con y sin gafete, y el gafete 0 cuenta como gafete", () => {
    expect(contarPorGafete(filas)).toEqual({ con: 2, sin: 2 });
  });

  it("sin filas, todo en cero", () => {
    expect(contarPorGafete([])).toEqual({ con: 0, sin: 0 });
  });
});

describe("filtrarPorGafete", () => {
  it("todos devuelve todas las filas", () => {
    expect(filtrarPorGafete(filas, "todos").map((f) => f.id)).toEqual([1, 2, 3, 4]);
  });

  it("sin deja sólo los que entraron sin gafete (S/G)", () => {
    expect(filtrarPorGafete(filas, "sin").map((f) => f.id)).toEqual([2, 4]);
  });

  // El gafete 0 es un número real, no "sin gafete" -- no confundirlo con nulo.
  it("con deja sólo los que tienen gafete, incluido el número 0", () => {
    expect(filtrarPorGafete(filas, "con").map((f) => f.id)).toEqual([1, 3]);
  });
});

describe("masDeDoceHoras", () => {
  const ahora = new Date("2026-09-23T20:00:00Z").getTime();

  it("resalta a quien lleva mas de 12 horas adentro", () => {
    expect(masDeDoceHoras({ fecha_hora_ingreso: "2026-09-23T07:59:00Z" }, ahora)).toBe(true);
  });

  it("12 horas justas o menos no se resalta", () => {
    expect(masDeDoceHoras({ fecha_hora_ingreso: "2026-09-23T08:00:00Z" }, ahora)).toBe(false);
    expect(masDeDoceHoras({ fecha_hora_ingreso: "2026-09-23T19:00:00Z" }, ahora)).toBe(false);
  });
});

describe("larga permanencia", () => {
  const ahora = new Date("2026-10-15T12:00:00Z").getTime();
  const hace = (horas: number) => ({
    fecha_hora_ingreso: new Date(ahora - horas * 3600_000).toISOString(),
  });

  it("cuenta sólo a quienes pasan de 12 horas", () => {
    expect(contarMasDeDoceHoras([hace(1), hace(11.9), hace(12.1), hace(30)], ahora)).toBe(2);
  });

  it("sin nadie con larga permanencia, cero", () => {
    expect(contarMasDeDoceHoras([hace(1), hace(5)], ahora)).toBe(0);
    expect(contarMasDeDoceHoras([], ahora)).toBe(0);
  });

  it("el texto va en singular o plural", () => {
    expect(textoAvisoPermanencia(1)).toBe("1 lleva más de 12 h adentro");
    expect(textoAvisoPermanencia(3)).toBe("3 llevan más de 12 h adentro");
  });
});
