import { describe, expect, it } from "vitest";
import type { TipoAdentro } from "../api/adentro";
import { contarPorClase, contarPorGafete, filtrarPorClase, filtrarPorGafete } from "./Activos.logica";

const fila = (tipo: TipoAdentro, gafete_numero: number | null) => ({ tipo, gafete_numero });

const filas = [
  fila("CONTRATISTA", 4),
  fila("CONTRATISTA", null),
  fila("CONTRATISTA", 9),
  fila("PROVEEDOR", 7),
  fila("PROVISIONAL_KOF", 2),
];

describe("filtro por clase", () => {
  it("todas deja todo y una clase deja sólo esa", () => {
    expect(filtrarPorClase(filas, "todas")).toHaveLength(5);
    expect(filtrarPorClase(filas, "CONTRATISTA")).toHaveLength(3);
    expect(filtrarPorClase(filas, "POR_CORREO")).toEqual([]);
  });

  it("cuenta cada clase, también las que no tienen a nadie", () => {
    expect(contarPorClase(filas)).toEqual({ CONTRATISTA: 3, PROVEEDOR: 1, POR_CORREO: 0, PROVISIONAL_KOF: 1 });
  });
});

describe("filtro por gafete", () => {
  const contratistas = filtrarPorClase(filas, "CONTRATISTA");

  it("con, sin (S/G) y todos", () => {
    expect(filtrarPorGafete(contratistas, "sin")).toEqual([fila("CONTRATISTA", null)]);
    expect(filtrarPorGafete(contratistas, "con")).toHaveLength(2);
    expect(filtrarPorGafete(contratistas, "todos")).toHaveLength(3);
  });

  it("cuenta con y sin gafete", () => {
    expect(contarPorGafete(contratistas)).toEqual({ con: 2, sin: 1 });
  });
});
