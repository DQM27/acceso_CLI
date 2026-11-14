import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SeccionActivaProvider } from "../contexto/SeccionActivaContexto";
import { useAutoRefresh } from "./useAutoRefresh";

const mocks = vi.hoisted(() => ({ channel: vi.fn(), removeChannel: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: mocks }));
let aviso: () => void;

beforeEach(() => {
  vi.useFakeTimers();
  vi.clearAllMocks();
  vi.spyOn(document, "hidden", "get").mockReturnValue(false);
  const canal = {
    on: vi.fn((_tipo, _filtro, callback) => {
      aviso = callback;
      return canal;
    }),
    subscribe: vi.fn(() => canal),
  };
  mocks.channel.mockReturnValue(canal);
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
});

/** Monta el hook dentro de una sección que se puede mostrar u ocultar. */
function montar(recargar: () => void) {
  let activa = true;
  const hook = renderHook(() => useAutoRefresh(recargar, 30_000, "ingresos"), {
    wrapper: ({ children }: { children: ReactNode }) => (
      <SeccionActivaProvider value={activa}>{children}</SeccionActivaProvider>
    ),
  });
  return {
    ...hook,
    mostrar(valor: boolean) {
      activa = valor;
      hook.rerender();
    },
  };
}

describe("refresco del panel con la sección oculta", () => {
  it("oculta, ni el intervalo ni Realtime piden datos", async () => {
    const recargar = vi.fn();
    const seccion = montar(recargar);
    seccion.mostrar(false);
    act(() => aviso());
    await act(() => vi.advanceTimersByTimeAsync(90_000));
    expect(recargar).not.toHaveBeenCalled();
    seccion.unmount();
  });

  it("al volver a mostrarse con algo pendiente, refresca una sola vez", async () => {
    const recargar = vi.fn();
    const seccion = montar(recargar);
    seccion.mostrar(false);
    act(() => aviso());
    await act(() => vi.advanceTimersByTimeAsync(60_000));
    seccion.mostrar(true);
    await act(() => vi.advanceTimersByTimeAsync(300));
    expect(recargar).toHaveBeenCalledTimes(1);
    seccion.unmount();
  });

  it("al volver sin nada pendiente, no refresca", async () => {
    const recargar = vi.fn();
    const seccion = montar(recargar);
    seccion.mostrar(false);
    seccion.mostrar(true);
    await act(() => vi.advanceTimersByTimeAsync(300));
    expect(recargar).not.toHaveBeenCalled();
    seccion.unmount();
  });
});
