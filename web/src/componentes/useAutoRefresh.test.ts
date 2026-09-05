import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useAutoRefresh } from "./useAutoRefresh";

const mocks = vi.hoisted(() => ({ channel: vi.fn(), removeChannel: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: mocks }));
let aviso: () => void;
let estado: (valor: string) => void;
let desmontar: (() => void) | undefined;
beforeEach(() => {
  vi.useFakeTimers();
  vi.clearAllMocks();
  vi.spyOn(document, "hidden", "get").mockReturnValue(false);
  const canal = {
    on: vi.fn((_tipo, _filtro, callback) => { aviso = callback; return canal; }),
    subscribe: vi.fn((callback) => { estado = callback; return canal; }),
  };
  mocks.channel.mockReturnValue(canal);
});
afterEach(() => { desmontar?.(); vi.restoreAllMocks(); vi.useRealTimers(); });

describe("recarga en vivo del panel", () => {
  it("agrupa eventos y usa la recarga vigente sin recrear el canal", async () => {
    const primera = vi.fn(), segunda = vi.fn();
    const hook = renderHook(({ recargar }) => useAutoRefresh(recargar, 30_000, "ingresos"), { initialProps: { recargar: primera } });
    desmontar = hook.unmount;
    hook.rerender({ recargar: segunda });
    act(() => { aviso(); aviso(); estado("SUBSCRIBED"); });
    await act(() => vi.advanceTimersByTimeAsync(300));
    expect(primera).not.toHaveBeenCalled();
    expect(segunda).toHaveBeenCalledTimes(1);
    expect(mocks.channel).toHaveBeenCalledTimes(1);
  });
  it("recupera cambios al volver a la pestaña y libera el canal al salir", async () => {
    const recargar = vi.fn();
    const hook = renderHook(() => useAutoRefresh(recargar, 30_000, "ingresos"));
    desmontar = hook.unmount;
    vi.spyOn(document, "hidden", "get").mockReturnValue(true);
    act(() => aviso());
    await act(() => vi.advanceTimersByTimeAsync(30_000));
    expect(recargar).not.toHaveBeenCalled();
    vi.spyOn(document, "hidden", "get").mockReturnValue(false);
    act(() => document.dispatchEvent(new Event("visibilitychange")));
    await act(() => vi.advanceTimersByTimeAsync(300));
    expect(recargar).toHaveBeenCalledTimes(1);
    hook.unmount();
    expect(mocks.removeChannel).toHaveBeenCalledTimes(1);
    await act(() => vi.advanceTimersByTimeAsync(30_000));
    expect(recargar).toHaveBeenCalledTimes(1);
  });
});
