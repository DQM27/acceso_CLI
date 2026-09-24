import { describe, expect, it } from "vitest";
import { filtrarPorGafete, masDeDoceHoras } from "./Activos";

const filas = [
  { id: 1, gafete_numero: 10 },
  { id: 2, gafete_numero: null },
  { id: 3, gafete_numero: 0 },
  { id: 4, gafete_numero: null },
];

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
