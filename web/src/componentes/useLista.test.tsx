import type { ReactNode } from "react";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useLista } from "./useLista";

// useAutoRefresh ya tiene su propio test; acá sólo importa que useLista lo arme
// con la tabla y el intervalo pedidos (usa Realtime de Supabase por dentro).
const mocks = vi.hoisted(() => ({ useAutoRefresh: vi.fn(), toastError: vi.fn() }));
vi.mock("./useAutoRefresh", () => ({ useAutoRefresh: mocks.useAutoRefresh }));
vi.mock("sonner", () => ({ toast: { error: mocks.toastError } }));

function envoltorio() {
  const cliente = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const Proveedor = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={cliente}>{children}</QueryClientProvider>
  );
  return { cliente, Proveedor };
}

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("useLista", () => {
  it("carga los datos y avisa cargando mientras tanto", async () => {
    const consulta = vi.fn().mockResolvedValue({ filas: [1, 2] });
    const { Proveedor } = envoltorio();

    const { result } = renderHook(() => useLista(["prueba"], consulta, { intervaloMs: 1000, tablas: "t" }), {
      wrapper: Proveedor,
    });

    expect(result.current.cargando).toBe(true);
    expect(result.current.datos).toBeUndefined();
    await waitFor(() => expect(result.current.datos).toEqual({ filas: [1, 2] }));
    expect(result.current.cargando).toBe(false);
  });

  it("le pide a useAutoRefresh el intervalo y las tablas indicados", async () => {
    const { Proveedor } = envoltorio();

    renderHook(() => useLista(["prueba"], vi.fn().mockResolvedValue({}), { intervaloMs: 120_000, tablas: "usuarios" }), {
      wrapper: Proveedor,
    });

    expect(mocks.useAutoRefresh).toHaveBeenCalledWith(expect.any(Function), 120_000, "usuarios");
  });

  it("recargar vuelve a pedir los datos y espera a tenerlos", async () => {
    const consulta = vi.fn().mockResolvedValueOnce({ n: 1 }).mockResolvedValueOnce({ n: 2 });
    const { Proveedor } = envoltorio();
    const { result } = renderHook(() => useLista(["prueba"], consulta, { intervaloMs: 1000, tablas: "t" }), {
      wrapper: Proveedor,
    });
    await waitFor(() => expect(result.current.datos).toEqual({ n: 1 }));

    await result.current.recargar();

    expect(consulta).toHaveBeenCalledTimes(2);
    await waitFor(() => expect(result.current.datos).toEqual({ n: 2 }));
  });

  it("avisa el error si todavía no hay datos", async () => {
    const consulta = vi.fn().mockRejectedValue(new Error("sin red"));
    const { Proveedor } = envoltorio();

    renderHook(() => useLista(["prueba"], consulta, { intervaloMs: 1000, tablas: "t" }), { wrapper: Proveedor });

    await waitFor(() => expect(mocks.toastError).toHaveBeenCalledTimes(1));
  });

  it("no molesta con un aviso si un refresco en segundo plano falla y ya había datos", async () => {
    const consulta = vi.fn().mockResolvedValueOnce({ n: 1 }).mockRejectedValueOnce(new Error("sin red"));
    const { Proveedor } = envoltorio();
    const { result } = renderHook(() => useLista(["prueba"], consulta, { intervaloMs: 1000, tablas: "t" }), {
      wrapper: Proveedor,
    });
    await waitFor(() => expect(result.current.datos).toEqual({ n: 1 }));

    await result.current.recargar();

    expect(result.current.datos).toEqual({ n: 1 });
    expect(mocks.toastError).not.toHaveBeenCalled();
  });
});
