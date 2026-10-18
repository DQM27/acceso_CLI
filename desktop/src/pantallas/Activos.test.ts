import { describe, expect, it } from "vitest";
import {
  contarPorClase,
  contarPorGafete,
  ETIQUETA_CLASE,
  filtrarPorClase,
  filtrarPorGafete,
  filtrarPorNivel,
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

describe("filtrarPorClase", () => {
  const filas = [
    { id: 1, clase: "contratista" as const },
    { id: 2, clase: "proveedor" as const },
    { id: 3, clase: "contratista" as const },
    { id: 4, clase: "kof" as const },
    { id: 5, clase: "correo" as const },
  ];

  it("todas devuelve todas las filas", () => {
    expect(filtrarPorClase(filas, "todas").map((f) => f.id)).toEqual([1, 2, 3, 4, 5]);
  });

  it("una clase deja sólo sus filas", () => {
    expect(filtrarPorClase(filas, "contratista").map((f) => f.id)).toEqual([1, 3]);
    expect(filtrarPorClase(filas, "kof").map((f) => f.id)).toEqual([4]);
  });

  it("una clase sin filas da una lista vacía", () => {
    expect(filtrarPorClase(filas.slice(0, 1), "proveedor")).toEqual([]);
  });
});

describe("contarPorClase", () => {
  it("cuenta cada clase y deja en cero las que no tienen filas", () => {
    const filas = [
      { clase: "contratista" as const },
      { clase: "contratista" as const },
      { clase: "kof" as const },
    ];
    expect(contarPorClase(filas)).toEqual({ contratista: 2, proveedor: 0, correo: 0, kof: 1 });
  });

  it("sin filas, todo en cero", () => {
    expect(contarPorClase([])).toEqual({ contratista: 0, proveedor: 0, correo: 0, kof: 0 });
  });
});

describe("ETIQUETA_CLASE", () => {
  it("tiene un nombre para cada clase", () => {
    expect(Object.keys(ETIQUETA_CLASE).sort()).toEqual(["contratista", "correo", "kof", "proveedor"]);
  });
});

describe("filtrarPorNivel con filas sin nivel", () => {
  const filas = [
    { id: 1, nivel_permanencia: "Critico" },
    { id: 2, nivel_permanencia: null },
  ];

  // Sólo los contratistas tienen nivel: con un aviso pulsado, el resto no entra.
  it("las filas sin nivel no coinciden con ningún aviso", () => {
    expect(filtrarPorNivel(filas, "Critico").map((f) => f.id)).toEqual([1]);
    expect(filtrarPorNivel(filas, "Aviso")).toEqual([]);
  });

  it("sin aviso pulsado se muestran todas, también las sin nivel", () => {
    expect(filtrarPorNivel(filas, null).map((f) => f.id)).toEqual([1, 2]);
  });
});
