import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { editarUsuario, listarUsuarios } from "./usuarios";

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

describe("listarUsuarios", () => {
  it("truncado en false cuando el conteo real coincide con lo que vino", async () => {
    const filas = [{ id: "1", cedula: "123", nombre: "Alguien", rol: "OPERADOR", activo: true }];
    mocks.from.mockReturnValue(mockConsulta({ data: filas, error: null, count: 1 }));

    const resultado = await listarUsuarios();

    expect(resultado.truncado).toBe(false);
    expect(resultado.filas).toEqual(filas);
  });

  it("truncado en true cuando el conteo real es mayor que las filas devueltas (tope alcanzado)", async () => {
    const filas = Array.from({ length: 2 }, (_, i) => ({
      id: String(i),
      cedula: String(i),
      nombre: `Fila ${i}`,
      rol: "OPERADOR" as const,
      activo: true,
    }));
    mocks.from.mockReturnValue(mockConsulta({ data: filas, error: null, count: 20_000 }));

    const resultado = await listarUsuarios();

    expect(resultado.truncado).toBe(true);
    expect(resultado.filas).toHaveLength(2);
  });

  it("propaga el error de la consulta como Error real", async () => {
    mocks.from.mockReturnValue(
      mockConsulta({ data: null, error: { message: "sin permiso" }, count: null }),
    );

    await expect(listarUsuarios()).rejects.toThrow("sin permiso");
  });

  it("lanza un error de validación si Supabase devuelve una fila con forma inesperada", async () => {
    const filas = [{ id: "1", cedula: "123", nombre: "Alguien", rol: "SUPERADMIN", activo: true }];
    mocks.from.mockReturnValue(mockConsulta({ data: filas, error: null, count: 1 }));

    await expect(listarUsuarios()).rejects.toThrow();
  });
});

describe("editarUsuario", () => {
  function mockUpdate(error: unknown = null) {
    const eq = vi.fn().mockResolvedValue({ error });
    const update = vi.fn(() => ({ eq }));
    mocks.from.mockReturnValue({ update });
    return { update, eq };
  }

  it("actualiza nombre (recortado) y rol de ese usuario", async () => {
    const { update, eq } = mockUpdate();
    await editarUsuario("u1", { nombre: "  Ana ", rol: "ADMINISTRADOR" });
    expect(mocks.from).toHaveBeenCalledWith("usuarios");
    expect(update).toHaveBeenCalledWith({ nombre: "Ana", rol: "ADMINISTRADOR" });
    expect(eq).toHaveBeenCalledWith("id", "u1");
  });

  it("sin rol (un ROOT) sólo cambia el nombre, nunca la cédula", async () => {
    const { update } = mockUpdate();
    await editarUsuario("u1", { nombre: "Ana" });
    expect(update).toHaveBeenCalledWith({ nombre: "Ana" });
  });

  it("no manda un nombre vacío, y propaga el error de la base", async () => {
    mockUpdate();
    await expect(editarUsuario("u1", { nombre: "   " })).rejects.toThrow("El nombre es obligatorio");
    mockUpdate({ message: "permission denied" });
    await expect(editarUsuario("u1", { nombre: "Ana" })).rejects.toThrow("permission denied");
  });
});
