import { describe, expect, it } from "vitest";
import { mensajeBloqueoVisita, puedeContinuarVisita } from "./citas";
import type { PreparacionVisita } from "./citas";

function preparacion(overrides: Partial<PreparacionVisita> = {}): PreparacionVisita {
  return {
    cita: {
      id: 1,
      motivo: null,
      fecha_desde: "2026-08-10",
      fecha_hasta: "2026-08-15",
      hora_estimada: null,
      anfitrion_nombre: "Anfitrión",
      anfitrion_correo: "anfitrion@ejemplo.com",
      estado: "Vigente",
    },
    visitante: {
      id: 1,
      cita_id: 1,
      cedula: "1-2345",
      nombre: "Visitante",
      empresa: null,
      placa_vehiculo: null,
    },
    activo_en_otro_sitio: null,
    ...overrides,
  };
}

describe("puedeContinuarVisita", () => {
  it("permite continuar sin conflicto", () => {
    expect(puedeContinuarVisita(preparacion())).toBe(true);
  });

  it("no permite continuar si ya está activo en otro sitio", () => {
    expect(puedeContinuarVisita(preparacion({ activo_en_otro_sitio: "Cartago" }))).toBe(false);
  });
});

describe("mensajeBloqueoVisita", () => {
  it("nombra al visitante y el sitio en conflicto", () => {
    expect(
      mensajeBloqueoVisita(preparacion({ activo_en_otro_sitio: "Cartago" })),
    ).toBe("Visitante ya tiene un movimiento activo en Cartago.");
  });
});
