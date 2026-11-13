import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const buscarContratistas = vi.fn();
vi.mock("../api", () => ({ buscarContratistas: (filtro: { texto: string }) => buscarContratistas(filtro) }));

import { ESPERA_BUSQUEDA_MS, useBusquedaContratistas } from "./useBusquedaContratistas";

/** Respuesta que se resuelve a mano, para decidir en qué orden llegan. */
function pendiente() {
  let resolver: (valor: unknown) => void = () => {};
  const promesa = new Promise((r) => {
    resolver = r;
  });
  return { promesa, resolver };
}

const pagina = (nombre: string) => ({ items: [{ id: 1, nombre }] });

beforeEach(() => {
  vi.useFakeTimers();
  buscarContratistas.mockReset();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useBusquedaContratistas", () => {
  it("una respuesta vieja que llega tarde no pisa la del texto actual", async () => {
    const lenta = pendiente();
    const rapida = pendiente();
    buscarContratistas.mockReturnValueOnce(lenta.promesa).mockReturnValueOnce(rapida.promesa);

    const { result, rerender } = renderHook(({ texto }) => useBusquedaContratistas(texto, true, () => {}), {
      initialProps: { texto: "car" },
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(ESPERA_BUSQUEDA_MS);
    });
    rerender({ texto: "carlos" });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(ESPERA_BUSQUEDA_MS);
    });

    await act(async () => {
      rapida.resolver(pagina("CARLOS"));
      await Promise.resolve();
    });
    await act(async () => {
      lenta.resolver(pagina("CAROLINA"));
      await Promise.resolve();
    });

    expect(result.current.resultados.map((c) => c.nombre)).toEqual(["CARLOS"]);
    expect(result.current.buscando).toBe(false);
  });

  it("mientras espera la respuesta del texto nuevo, marca que está buscando", async () => {
    buscarContratistas.mockReturnValue(new Promise(() => {}));
    const { result } = renderHook(() => useBusquedaContratistas("ana", true, () => {}));
    expect(result.current.buscando).toBe(true);
  });

  it("inactiva o sin texto, no consulta y queda vacía", async () => {
    const { result, rerender } = renderHook(({ texto, activa }) => useBusquedaContratistas(texto, activa, () => {}), {
      initialProps: { texto: "ana", activa: false },
    });
    rerender({ texto: "   ", activa: true });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(ESPERA_BUSQUEDA_MS * 2);
    });
    expect(buscarContratistas).not.toHaveBeenCalled();
    expect(result.current).toEqual({ resultados: [], buscando: false });
  });

  it("si la consulta falla, avisa y deja de buscar", async () => {
    buscarContratistas.mockRejectedValue(new Error("base ocupada"));
    const alFallar = vi.fn();
    const { result } = renderHook(() => useBusquedaContratistas("ana", true, alFallar));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(ESPERA_BUSQUEDA_MS);
    });
    expect(alFallar).toHaveBeenCalledWith("Error: base ocupada");
    expect(result.current.buscando).toBe(false);
  });
});
