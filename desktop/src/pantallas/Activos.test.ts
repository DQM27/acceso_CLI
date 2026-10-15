import { describe, expect, it } from "vitest";
import { contarPorGafete, filtrarPorGafete, filtrarPorNivel } from "./Activos.logica";

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

describe("filtrarPorNivel", () => {
  const filas = [
    { id: 1, nivel_permanencia: "Normal" },
    { id: 2, nivel_permanencia: "Aviso" },
    { id: 3, nivel_permanencia: "Critico" },
    { id: 4, nivel_permanencia: "Aviso" },
  ];

  it("sin nivel devuelve todas las filas", () => {
    expect(filtrarPorNivel(filas, null).map((f) => f.id)).toEqual([1, 2, 3, 4]);
  });

  it("deja sólo las del nivel que marcó el núcleo", () => {
    expect(filtrarPorNivel(filas, "Aviso").map((f) => f.id)).toEqual([2, 4]);
    expect(filtrarPorNivel(filas, "Critico").map((f) => f.id)).toEqual([3]);
  });
});
