import { renderHook, waitFor } from "@testing-library/react";
import { avisos } from "../avisos/avisos";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useCargaAlCambiar } from "./useCargaAlCambiar";

describe("useCargaAlCambiar", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("corre recargar() al montar", () => {
    const recargar = vi.fn().mockResolvedValue(undefined);
    renderHook(() => useCargaAlCambiar(recargar));
    expect(recargar).toHaveBeenCalledTimes(1);
  });

  it("si recargar() falla, avisa con avisos.error", async () => {
    const spy = vi.spyOn(avisos, "error").mockImplementation(() => {});
    const recargar = vi.fn().mockRejectedValue(new Error("sin conexión"));

    renderHook(() => useCargaAlCambiar(recargar));

    await waitFor(() => expect(spy).toHaveBeenCalledWith(new Error("sin conexión")));
  });

  it("si la pantalla se desmontó antes de que falle, NO avisa", async () => {
    const spy = vi.spyOn(avisos, "error").mockImplementation(() => {});
    let rechazar: (error: unknown) => void = () => {};
    const recargar = vi.fn(
      () =>
        new Promise((_resolve, reject) => {
          rechazar = reject;
        }),
    );

    const { unmount } = renderHook(() => useCargaAlCambiar(recargar));
    unmount();
    rechazar(new Error("llegó tarde"));

    // Deja pasar el microtask del .catch() sin que haya nada que esperar
    // con waitFor (justamente se espera que NO pase nada).
    await Promise.resolve();
    expect(spy).not.toHaveBeenCalled();
  });

  it("vuelve a correr recargar() cuando cambia su identidad", () => {
    const recargarA = vi.fn().mockResolvedValue(undefined);
    const recargarB = vi.fn().mockResolvedValue(undefined);

    const { rerender } = renderHook(
      ({ recargar }: { recargar: () => Promise<void> }) => useCargaAlCambiar(recargar),
      { initialProps: { recargar: recargarA } },
    );
    expect(recargarA).toHaveBeenCalledTimes(1);

    rerender({ recargar: recargarB });
    expect(recargarB).toHaveBeenCalledTimes(1);
    // recargarA no se vuelve a llamar sólo porque cambió la prop.
    expect(recargarA).toHaveBeenCalledTimes(1);
  });
});
