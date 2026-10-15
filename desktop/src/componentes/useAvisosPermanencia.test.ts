import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const avisosPermanencia = vi.fn();
vi.mock("../api", () => ({ avisosPermanencia: () => avisosPermanencia() }));

import { useAvisosPermanencia } from "./useAvisosPermanencia";

const respuesta = {
  en_aviso: 2,
  criticos: 0,
  texto_aviso: "2 llevan más de 12 h adentro",
  texto_critico: null,
};

beforeEach(() => {
  avisosPermanencia.mockReset();
  avisosPermanencia.mockResolvedValue(respuesta);
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useAvisosPermanencia", () => {
  it("guarda lo que contesta el núcleo, sin calcular nada", async () => {
    const { result } = renderHook(() => useAvisosPermanencia(0));
    await waitFor(() => expect(result.current).toEqual(respuesta));
  });

  it("vuelve a preguntar cuando cambia la señal de recarga", async () => {
    const { rerender } = renderHook(({ senal }) => useAvisosPermanencia(senal), {
      initialProps: { senal: 0 },
    });
    await waitFor(() => expect(avisosPermanencia).toHaveBeenCalledTimes(1));
    rerender({ senal: 1 });
    await waitFor(() => expect(avisosPermanencia).toHaveBeenCalledTimes(2));
  });

  it("vuelve a preguntar cada minuto", async () => {
    vi.useFakeTimers();
    renderHook(() => useAvisosPermanencia(0));
    expect(avisosPermanencia).toHaveBeenCalledTimes(1);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(avisosPermanencia).toHaveBeenCalledTimes(2);
  });

  it("si falla, no rompe y se queda sin aviso", async () => {
    avisosPermanencia.mockRejectedValue(new Error("sin sesión"));
    const { result } = renderHook(() => useAvisosPermanencia(0));
    await waitFor(() => expect(avisosPermanencia).toHaveBeenCalled());
    expect(result.current).toBeNull();
  });
});
