import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { listarHistorial, listarUnidadesOperativas } from "./historial";

function mockConsulta(resultado: { data: unknown; error: unknown; count: number | null }) {
  const encadenable: Record<string, unknown> = {
    select: vi.fn(() => encadenable),
    order: vi.fn(() => encadenable),
    range: vi.fn(() => encadenable),
    gte: vi.fn(() => encadenable),
    lt: vi.fn(() => encadenable),
    in: vi.fn(() => encadenable),
    returns: vi.fn(() => encadenable),
    then: (resolver: (valor: typeof resultado) => void) => resolver(resultado),
  };
  return encadenable;
}

function filaCruda(sobrescribir: Record<string, unknown> = {}) {
  return {
    id: "1",
    sitio_id: "s1",
    sitios: { nombre: "Brisas" },
    contratista_cedula: "001",
    contratista_nombre: "Alguien",
    empresa_nombre: "Brisas",
    tipo_ingreso: "PRAIND",
    medio_ingreso: "CAMINANDO",
    gafete_numero: 1,
    hora_entrada: "2026-08-20T14:30:00Z",
    hora_salida: null,
    usuario_entrada_nombre: "Quintana",
    usuario_salida_nombre: null,
    dispositivo_entrada: { tipo: "pc" },
    ...sobrescribir,
  };
}

const mocks = vi.hoisted(() => ({ from: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: { from: mocks.from } }));

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("listarHistorial", () => {
  it("truncado en false cuando el conteo real coincide con lo que vino, y aplana sitios/dispositivo", async () => {
    const filas = [filaCruda()];
    mocks.from.mockReturnValue(mockConsulta({ data: filas, error: null, count: 1 }));

    const resultado = await listarHistorial();

    expect(resultado.truncado).toBe(false);
    expect(resultado.filas).toHaveLength(1);
    expect(resultado.filas[0].sitio_nombre).toBe("Brisas");
    expect(resultado.filas[0].dispositivo_entrada_tipo).toBe("pc");
  });

  it("truncado en true cuando el conteo real es mayor que las filas devueltas (tope alcanzado)", async () => {
    const filas = [filaCruda({ id: "1" }), filaCruda({ id: "2" })];
    mocks.from.mockReturnValue(mockConsulta({ data: filas, error: null, count: 100_000 }));

    const resultado = await listarHistorial("2026-01-01", undefined);

    expect(resultado.truncado).toBe(true);
    expect(resultado.filas).toHaveLength(2);
  });

  it("sitio_nombre/dispositivo_entrada_tipo caen a null cuando vienen ausentes", async () => {
    const filas = [filaCruda({ sitios: null, dispositivo_entrada: null })];
    mocks.from.mockReturnValue(mockConsulta({ data: filas, error: null, count: 1 }));

    const resultado = await listarHistorial();

    expect(resultado.filas[0].sitio_nombre).toBeNull();
    expect(resultado.filas[0].dispositivo_entrada_tipo).toBeNull();
  });

  it("propaga el error de la consulta como Error real", async () => {
    mocks.from.mockReturnValue(
      mockConsulta({ data: null, error: { message: "timeout" }, count: null }),
    );

    await expect(listarHistorial()).rejects.toThrow("timeout");
  });

  it("lanza un error de validación si Supabase devuelve una fila con forma inesperada", async () => {
    const filas = [filaCruda({ gafete_numero: "12" })];
    mocks.from.mockReturnValue(mockConsulta({ data: filas, error: null, count: 1 }));

    await expect(listarHistorial()).rejects.toThrow();
  });

  it("convierte desde/hasta (YMD) a límites UTC de Costa Rica -- hasta es el inicio del día SIGUIENTE, exclusivo", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: 0 });
    mocks.from.mockReturnValue(encadenable);

    await listarHistorial("2026-09-09", "2026-09-09");

    // Antes se mandaba el YMD crudo: `lte("hora_entrada", "2026-09-09")` sólo
    // calificaba el instante exacto de esa medianoche UTC, dejando "Hoy"
    // prácticamente siempre vacío. Ahora "hasta" es exclusivo contra el
    // inicio del día siguiente en Costa Rica (UTC-6), así que el 9 de
    // septiembre completo (hasta las 23:59:59 hora local) queda adentro.
    expect(encadenable.gte).toHaveBeenCalledWith("hora_entrada", "2026-09-09T00:00:00-06:00");
    expect(encadenable.lt).toHaveBeenCalledWith("hora_entrada", "2026-09-10T00:00:00-06:00");
  });

  it("cruza el fin de mes al calcular el día siguiente para 'hasta'", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: 0 });
    mocks.from.mockReturnValue(encadenable);

    await listarHistorial(undefined, "2026-09-30");

    expect(encadenable.lt).toHaveBeenCalledWith("hora_entrada", "2026-10-01T00:00:00-06:00");
  });

  it("sin sitioIds no filtra por sitio", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: 0 });
    mocks.from.mockReturnValue(encadenable);

    await listarHistorial();

    expect(encadenable.in).not.toHaveBeenCalled();
  });

  it("con sitioIds filtra con .in(sitio_id, ...) -- una lista vacía trae cero filas a propósito (todas las unidades excluidas)", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: 0 });
    mocks.from.mockReturnValue(encadenable);

    await listarHistorial(undefined, undefined, ["s1", "s2"]);
    expect(encadenable.in).toHaveBeenCalledWith("sitio_id", ["s1", "s2"]);

    await listarHistorial(undefined, undefined, []);
    expect(encadenable.in).toHaveBeenCalledWith("sitio_id", []);
  });
});

describe("listarUnidadesOperativas", () => {
  it("devuelve id/nombre ordenados como los mandó Supabase", async () => {
    const sitios = [
      { id: "s1", nombre: "Brisas" },
      { id: "s2", nombre: "Otra unidad" },
    ];
    mocks.from.mockReturnValue(mockConsulta({ data: sitios, error: null, count: null }));

    const resultado = await listarUnidadesOperativas();

    expect(resultado).toEqual(sitios);
  });

  it("propaga el error de la consulta como Error real", async () => {
    mocks.from.mockReturnValue(
      mockConsulta({ data: null, error: { message: "timeout" }, count: null }),
    );

    await expect(listarUnidadesOperativas()).rejects.toThrow("timeout");
  });
});
