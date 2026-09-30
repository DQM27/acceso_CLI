import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { listarAdentroAhora, llevaDemasiado, textoTiempoAdentro } from "./adentro";

function mockConsulta(resultado: { data: unknown; error: unknown }) {
  const encadenable: Record<string, unknown> = {
    select: vi.fn(() => encadenable),
    order: vi.fn(() => encadenable),
    range: vi.fn(() => encadenable),
    then: (resolver: (valor: typeof resultado) => void) => resolver(resultado),
  };
  return encadenable;
}

const mocks = vi.hoisted(() => ({ from: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: { from: mocks.from } }));

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(() => {
  vi.restoreAllMocks();
});

function fila(sobrescribir: Record<string, unknown> = {}) {
  return {
    tipo: "PROVISIONAL_KOF",
    id: "k1",
    sitio_id: "s1",
    sitio_nombre: "Brisas",
    identificacion: "EMP-1",
    nombre: "ENCARGADO",
    empresa_nombre: "KOF",
    gafete_numero: 7,
    placa: null,
    hora_entrada: "2026-09-30T12:00:00Z",
    usuario_entrada_nombre: "OP",
    ...sobrescribir,
  };
}

describe("listarAdentroAhora", () => {
  it("lee panel_adentro_ahora ordenado por la entrada más antigua primero", async () => {
    const encadenable = mockConsulta({ data: [fila()], error: null });
    mocks.from.mockReturnValue(encadenable);

    const filas = await listarAdentroAhora();

    expect(mocks.from).toHaveBeenCalledWith("panel_adentro_ahora");
    expect(encadenable.order).toHaveBeenCalledWith("hora_entrada", { ascending: true });
    expect(filas[0].tipo).toBe("PROVISIONAL_KOF");
  });

  it("propaga el error y rechaza un tipo desconocido", async () => {
    mocks.from.mockReturnValue(mockConsulta({ data: null, error: { message: "sin permiso" } }));
    await expect(listarAdentroAhora()).rejects.toThrow("sin permiso");

    mocks.from.mockReturnValue(mockConsulta({ data: [fila({ tipo: "VISITA" })], error: null }));
    await expect(listarAdentroAhora()).rejects.toThrow();
  });
});

describe("tiempo adentro", () => {
  const desde = "2026-09-30T12:00:00Z";
  const mas = (minutos: number) => new Date(Date.parse(desde) + minutos * 60_000);

  it("minutos, horas y días en palabras cortas", () => {
    expect(textoTiempoAdentro(desde, mas(45))).toBe("45 min");
    expect(textoTiempoAdentro(desde, mas(180))).toBe("3 h");
    expect(textoTiempoAdentro(desde, mas(200))).toBe("3 h 20 min");
    expect(textoTiempoAdentro(desde, mas(24 * 60))).toBe("1 d");
    expect(textoTiempoAdentro(desde, mas(52 * 60))).toBe("2 d 4 h");
    // Reloj del equipo adelantado: nunca negativo.
    expect(textoTiempoAdentro(desde, mas(-5))).toBe("0 min");
  });

  it("resalta desde las 12 horas", () => {
    expect(llevaDemasiado(desde, mas(12 * 60 - 1))).toBe(false);
    expect(llevaDemasiado(desde, mas(12 * 60))).toBe(true);
  });
});
