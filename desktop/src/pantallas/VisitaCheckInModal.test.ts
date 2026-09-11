import { describe, expect, it } from "vitest";
import { estaYaAdentro, validarGafeteOpcional } from "./VisitaCheckInModal";
import type { MovimientoVisitaActivoResumen } from "../api";

function visitaActiva(overrides: Partial<MovimientoVisitaActivoResumen> = {}): MovimientoVisitaActivoResumen {
  return {
    id: 1,
    cedula: "1-0847-0293",
    nombre: "Marlon Quesada",
    empresa: null,
    anfitrion_nombre: "Ana Pérez",
    motivo: null,
    gafete_numero: null,
    fecha_hora_entrada: "2027-03-08T12:00:00Z",
    ...overrides,
  };
}

describe("validarGafeteOpcional", () => {
  it("vacío es válido, con numero null", () => {
    expect(validarGafeteOpcional("")).toEqual({ valido: true, numero: null });
    expect(validarGafeteOpcional("   ")).toEqual({ valido: true, numero: null });
  });

  it("no numérico es inválido", () => {
    expect(validarGafeteOpcional("abc")).toEqual({
      valido: false,
      mensaje: "Ingrese un número de gafete válido",
    });
  });

  it("numérico válido", () => {
    expect(validarGafeteOpcional("5")).toEqual({ valido: true, numero: 5 });
  });
});

describe("estaYaAdentro", () => {
  it("encuentra por cédula exacta", () => {
    const activas = [visitaActiva({ cedula: "1-0847-0293" })];
    expect(estaYaAdentro(activas, "1-0847-0293")).toBe(true);
  });

  it("no encuentra si la cédula no está en la lista", () => {
    const activas = [visitaActiva({ cedula: "1-0847-0293" })];
    expect(estaYaAdentro(activas, "2-1111-2222")).toBe(false);
  });

  it("lista vacía nunca da true", () => {
    expect(estaYaAdentro([], "1-0847-0293")).toBe(false);
  });
});
