import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { crearSitio, listarDispositivosYSitios } from "./dispositivos";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: { functions: { invoke: mocks.invoke } } }));

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("invocar (validación de forma de la respuesta)", () => {
  it("devuelve los datos tal cual cuando la forma es la esperada", async () => {
    const sitio = { id: "1", nombre: "Brisas", direccion: null, created_at: "2026-01-01" };
    mocks.invoke.mockResolvedValue({ data: sitio, error: null });

    await expect(crearSitio({ nombre: "Brisas" })).resolves.toEqual(sitio);
  });

  it("falla con un mensaje claro si el backend devuelve una forma distinta a la esperada, en vez de un cast silencioso", async () => {
    // Ej. la Edge Function cambió su contrato (renombró un campo) sin que
    // el panel se haya actualizado a la vez -- antes esto pasaba `as T` y
    // recién explotaba más abajo con un `undefined.algo` sin contexto.
    mocks.invoke.mockResolvedValue({ data: { nombreDelSitio: "Brisas" }, error: null });

    await expect(crearSitio({ nombre: "Brisas" })).rejects.toThrow(/forma inesperada/);
  });

  it("listarDispositivosYSitios rechaza si 'dispositivos' no es un array", async () => {
    mocks.invoke.mockResolvedValue({ data: { sitios: [], dispositivos: null }, error: null });

    await expect(listarDispositivosYSitios()).rejects.toThrow(/forma inesperada/);
  });

  it("propaga el detalle del error de la Edge Function cuando la invocación falla", async () => {
    const contexto = new Response(JSON.stringify({ detail: "sitio duplicado" }), { status: 409 });
    mocks.invoke.mockResolvedValue({
      data: null,
      error: Object.assign(new Error("Edge Function error"), { context: contexto }),
    });

    await expect(crearSitio({ nombre: "Brisas" })).rejects.toThrow("sitio duplicado");
  });
});
