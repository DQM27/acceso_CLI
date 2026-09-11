import { describe, expect, it } from "vitest";
import { finExclusivo, tituloEvento } from "./AgendaCalendario";
import type { AgendaVisitaResumen } from "../api";

function fila(overrides: Partial<AgendaVisitaResumen> = {}): AgendaVisitaResumen {
  return {
    cita_id: 1,
    cedula: "1-0847-0293",
    nombre: "Marlon Quesada",
    empresa: null,
    placa_vehiculo: null,
    motivo: null,
    anfitrion_nombre: "Ana Pérez",
    fecha_desde: "2027-03-10",
    fecha_hasta: "2027-03-10",
    hora_estimada: null,
    estado: "Vigente",
    ...overrides,
  };
}

describe("finExclusivo", () => {
  it("suma un día -- FullCalendar necesita el fin exclusivo, no el inclusivo real", () => {
    expect(finExclusivo("2027-03-10")).toBe("2027-03-11");
  });

  it("cruza correctamente el fin de mes", () => {
    expect(finExclusivo("2027-03-31")).toBe("2027-04-01");
  });

  it("cruza correctamente el fin de año", () => {
    expect(finExclusivo("2027-12-31")).toBe("2028-01-01");
  });
});

describe("tituloEvento", () => {
  it("con empresa, junta nombre y empresa", () => {
    expect(tituloEvento(fila({ nombre: "Marlon Quesada", empresa: "Constructora del Valle" }))).toBe(
      "Marlon Quesada · Constructora del Valle",
    );
  });

  it("sin empresa, sólo el nombre", () => {
    expect(tituloEvento(fila({ nombre: "Marlon Quesada", empresa: null }))).toBe("Marlon Quesada");
  });
});
