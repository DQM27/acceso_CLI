import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { listarContratistas } from "./contratistas";

/**
 * Construye un mock encadenable de la query de supabase-js
 * (`.from().select().order().range().returns()`) que resuelve al
 * `awaitearlo` -- mismo shape real que usa el cliente, sin levantar un
 * servidor de verdad.
 */
function mockConsulta(resultado: { data: unknown; error: unknown; count: number | null }) {
  const encadenable: Record<string, unknown> = {
    select: vi.fn(() => encadenable),
    order: vi.fn(() => encadenable),
    range: vi.fn(() => encadenable),
    returns: vi.fn(() => encadenable),
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

describe("listarContratistas", () => {
  it("truncado en false cuando el conteo real coincide con lo que vino", async () => {
    const filas = [{ id: "1", nombre: "Alguien" }];
    mocks.from.mockReturnValue(mockConsulta({ data: filas, error: null, count: 1 }));

    const resultado = await listarContratistas();

    expect(resultado.truncado).toBe(false);
    expect(resultado.filas).toEqual(filas);
  });

  it("truncado en true cuando el conteo real es mayor que las filas devueltas (tope alcanzado)", async () => {
    const filas = Array.from({ length: 3 }, (_, i) => ({ id: String(i), nombre: `Fila ${i}` }));
    // El conteo real (lo que devuelve Postgres con count:'exact') es mayor
    // que lo que vino en `data` -- exactamente lo que pasa cuando `.range()`
    // corta antes de llegar al final.
    mocks.from.mockReturnValue(mockConsulta({ data: filas, error: null, count: 50_000 }));

    const resultado = await listarContratistas();

    expect(resultado.truncado).toBe(true);
    expect(resultado.filas).toHaveLength(3);
  });

  it("propaga el error de la consulta como Error real", async () => {
    mocks.from.mockReturnValue(
      mockConsulta({ data: null, error: { message: "RLS denegó el acceso" }, count: null }),
    );

    await expect(listarContratistas()).rejects.toThrow("RLS denegó el acceso");
  });
});
