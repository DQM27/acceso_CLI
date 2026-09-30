import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MAXIMO_SESIONES, listarSesiones, textoDuracion, textoEstadoSesion } from "./sesiones";

function mockConsulta(resultado: { data: unknown; error: unknown }) {
  const encadenable: Record<string, unknown> = {
    select: vi.fn(() => encadenable),
    order: vi.fn(() => encadenable),
    range: vi.fn(() => encadenable),
    gte: vi.fn(() => encadenable),
    lt: vi.fn(() => encadenable),
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

function sesion(sobrescribir: Record<string, unknown> = {}) {
  return {
    id: 1,
    cedula: "900000301",
    nombre: "OPERADOR",
    dispositivo_etiqueta: "PC recepción",
    dispositivo_tipo: "pc",
    sitio_id: "s1",
    sitio_nombre: "Brisas",
    iniciada_en: "2026-09-30T12:00:00Z",
    cerrada_en: null,
    motivo_cierre: null,
    ultima_actividad: "2026-09-30T13:30:00Z",
    abierta: true,
    ...sobrescribir,
  };
}

describe("listarSesiones", () => {
  it("lee la bitácora del rango (días de Costa Rica), de la más reciente a la más vieja", async () => {
    const consulta = mockConsulta({ data: [sesion()], error: null });
    mocks.from.mockReturnValue(consulta);

    const resultado = await listarSesiones({ desde: "2026-09-01", hasta: "2026-09-30" });

    expect(mocks.from).toHaveBeenCalledWith("panel_bitacora_sesiones");
    expect(consulta.order).toHaveBeenNthCalledWith(1, "iniciada_en", { ascending: false });
    expect(consulta.gte).toHaveBeenCalledWith("iniciada_en", "2026-09-01T00:00:00-06:00");
    expect(consulta.lt).toHaveBeenCalledWith("iniciada_en", "2026-10-01T00:00:00-06:00");
    // Una fila de más para saber si el rango quedó truncado.
    expect(consulta.range).toHaveBeenCalledWith(0, MAXIMO_SESIONES);
    expect(resultado).toEqual({ filas: [sesion()], truncado: false });
  });

  it("marca truncado y corta en el máximo cuando llega una fila de más", async () => {
    const muchas = Array.from({ length: MAXIMO_SESIONES + 1 }, (_, i) => sesion({ id: i }));
    mocks.from.mockReturnValue(mockConsulta({ data: muchas, error: null }));

    const resultado = await listarSesiones({});

    expect(resultado.truncado).toBe(true);
    expect(resultado.filas).toHaveLength(MAXIMO_SESIONES);
  });

  it("propaga el error y rechaza un motivo desconocido", async () => {
    mocks.from.mockReturnValue(mockConsulta({ data: null, error: { message: "sin permiso" } }));
    await expect(listarSesiones({})).rejects.toThrow("sin permiso");

    mocks.from.mockReturnValue(mockConsulta({ data: [sesion({ motivo_cierre: "inventado" })], error: null }));
    await expect(listarSesiones({})).rejects.toThrow();
  });
});

describe("textos", () => {
  it("estado: abierta o el motivo del cierre", () => {
    expect(textoEstadoSesion({ abierta: true, motivo_cierre: null })).toBe("Abierta");
    expect(textoEstadoSesion({ abierta: false, motivo_cierre: "salida" })).toBe("Salió");
    expect(textoEstadoSesion({ abierta: false, motivo_cierre: "otra_unidad" })).toBe("Entró en otra unidad");
    expect(textoEstadoSesion({ abierta: false, motivo_cierre: "desplazada" })).toBe("Desplazada por otra unidad");
    expect(textoEstadoSesion({ abierta: false, motivo_cierre: "sin_cierre" })).toBe("Sin cierre registrado");
  });

  it("duración en minutos, horas y días", () => {
    const inicio = "2026-09-30T12:00:00Z";
    const mas = (min: number) => new Date(Date.parse(inicio) + min * 60_000).toISOString();
    expect(textoDuracion(inicio, mas(45))).toBe("45 min");
    expect(textoDuracion(inicio, mas(90))).toBe("1 h 30 min");
    expect(textoDuracion(inicio, mas(120))).toBe("2 h");
    expect(textoDuracion(inicio, mas(26 * 60))).toBe("1 d 2 h");
    expect(textoDuracion(inicio, mas(-5))).toBe("0 min");
  });
});
