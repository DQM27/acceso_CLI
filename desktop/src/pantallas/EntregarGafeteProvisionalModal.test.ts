import { describe, expect, it } from "vitest";
import { filtrarEncargados } from "./EntregarGafeteProvisionalModal";
import type { EncargadoRuta } from "../api/rutas";

const encargado = (id: number, nombre: string, codigo_empleado: string): EncargadoRuta => ({
  id,
  nombre,
  codigo_empleado,
  cedula: null,
  activo: true,
});

const CATALOGO = [
  encargado(1, "JOSÉ PÉREZ MORA", "E-1001"),
  encargado(2, "MARÍA JOSÉ ROJAS", "E-2002"),
  encargado(3, "CARLOS VEGA", "E-3003"),
];

describe("filtrarEncargados", () => {
  it("busca por nombre sin distinguir tildes ni mayúsculas", () => {
    expect(filtrarEncargados(CATALOGO, "jose").map((e) => e.id)).toEqual([1, 2]);
  });

  it("busca por código de empleado", () => {
    expect(filtrarEncargados(CATALOGO, "3003").map((e) => e.id)).toEqual([3]);
  });

  it("exige todas las palabras", () => {
    expect(filtrarEncargados(CATALOGO, "jose rojas").map((e) => e.id)).toEqual([2]);
  });

  it("texto vacío no devuelve nada", () => {
    expect(filtrarEncargados(CATALOGO, "   ")).toEqual([]);
  });

  it("respeta el máximo de resultados", () => {
    expect(filtrarEncargados(CATALOGO, "e-", 2)).toHaveLength(2);
  });
});
