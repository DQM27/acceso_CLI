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

  it("avisa cuando el reloj encuentra que alguien cruzó un umbral", async () => {
    vi.useFakeTimers();
    const alCambiar = vi.fn();
    renderHook(() => useAvisosPermanencia(0, alCambiar));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    avisosPermanencia.mockResolvedValue({ ...respuesta, criticos: 1, texto_critico: "1 crítico" });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(alCambiar).toHaveBeenCalledTimes(1);
  });

  it("no avisa si la respuesta del minuto es la misma", async () => {
    vi.useFakeTimers();
    const alCambiar = vi.fn();
    renderHook(() => useAvisosPermanencia(0, alCambiar));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(120_000);
    });
    expect(avisosPermanencia).toHaveBeenCalledTimes(3);
    expect(alCambiar).not.toHaveBeenCalled();
  });

  it("no avisa si el cambio llegó por una recarga (los datos ya son nuevos)", async () => {
    const alCambiar = vi.fn();
    const { rerender } = renderHook(({ senal }) => useAvisosPermanencia(senal, alCambiar), {
      initialProps: { senal: 0 },
    });
    await waitFor(() => expect(avisosPermanencia).toHaveBeenCalledTimes(1));
    avisosPermanencia.mockResolvedValue({ ...respuesta, en_aviso: 0, texto_aviso: null });
    rerender({ senal: 1 });
    await waitFor(() => expect(avisosPermanencia).toHaveBeenCalledTimes(2));
    expect(alCambiar).not.toHaveBeenCalled();
  });

  it("si falla, no rompe y se queda sin aviso", async () => {
    avisosPermanencia.mockRejectedValue(new Error("sin sesión"));
    const { result } = renderHook(() => useAvisosPermanencia(0));
    await waitFor(() => expect(avisosPermanencia).toHaveBeenCalled());
    expect(result.current).toBeNull();
  });
});
