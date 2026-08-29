import { describe, expect, it } from "vitest";
import { coincideTexto } from "./SalidaModal";
import type { IngresoActivoResumen } from "../api";

function activo(overrides: Partial<IngresoActivoResumen> = {}): IngresoActivoResumen {
  return {
    registro_id: 1,
    contratista_id: 1,
    cedula: "1-0847-0293",
    contratista_nombre: "Marlon Quesada",
    empresa_nombre: "Constructora del Valle",
    tipo_ingreso: "Praind",
    medio_ingreso: "Caminando",
    fecha_hora_ingreso: "2027-03-08T12:00:00Z",
    gafete_numero: null,
    usuario_ingreso_nombre: "root",
    resultado_registrado: "Permitido",
    resultado_acceso: "Permitido",
    ...overrides,
  };
}

describe("coincideTexto", () => {
  it("busca por nombre, sin importar mayúsculas", () => {
    expect(coincideTexto(activo({ contratista_nombre: "Marlon Quesada" }), "marlon")).toBe(true);
    expect(coincideTexto(activo({ contratista_nombre: "Marlon Quesada" }), "MARLON")).toBe(true);
  });

  it("busca por cédula", () => {
    expect(coincideTexto(activo({ cedula: "1-0847-0293" }), "0847")).toBe(true);
  });

  it("coincidencia parcial en cualquier posición", () => {
    expect(coincideTexto(activo({ contratista_nombre: "Marlon Quesada" }), "esada")).toBe(true);
  });

  it("sin coincidencia en ninguno de los dos campos", () => {
    expect(coincideTexto(activo({ contratista_nombre: "Marlon Quesada", cedula: "1-0847-0293" }), "yuliana")).toBe(
      false,
    );
  });
});
