import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useDebounced } from "./useDebounced";

beforeEach(() => {
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
});

describe("useDebounced", () => {
  it("no actualiza el valor devuelto hasta que pasa el tiempo de espera sin cambios nuevos", () => {
    const hook = renderHook(({ valor }) => useDebounced(valor, 300), { initialProps: { valor: "a" } });
    expect(hook.result.current).toBe("a");

    hook.rerender({ valor: "ab" });
    act(() => vi.advanceTimersByTime(299));
    // Todavía no pasó el tiempo completo -- sigue en el valor viejo.
    expect(hook.result.current).toBe("a");

    act(() => vi.advanceTimersByTime(1));
    expect(hook.result.current).toBe("ab");
  });

  it("una racha de cambios rápidos sólo aplica el último valor, no uno por cada tecla", () => {
    const hook = renderHook(({ valor }) => useDebounced(valor, 300), { initialProps: { valor: "" } });

    for (const letra of ["a", "ab", "abc", "abcd"]) {
      hook.rerender({ valor: letra });
      act(() => vi.advanceTimersByTime(100)); // menos que los 300ms, cada vez reinicia el timer
    }
    expect(hook.result.current).toBe(""); // todavía nada aplicado

    act(() => vi.advanceTimersByTime(300));
    expect(hook.result.current).toBe("abcd");
  });
});
