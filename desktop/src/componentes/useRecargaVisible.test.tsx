import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it } from "vitest";
import { SeccionActivaProvider } from "../contexto/BarraEstadoContexto";
import { EVENTO_CAMBIO_LOCAL_NUBE, EVENTO_NUBE_ACTUALIZADA } from "../eventosNube";
import { useRecargaVisible } from "./useRecargaVisible";

/** Sección fija (siempre a la vista o siempre oculta). */
function montar(inicial: { senal: number; activa: boolean }) {
  return renderHook(({ senal }: { senal: number; activa: boolean }) => useRecargaVisible(senal), {
    initialProps: inicial,
    wrapper: ({ children }: { children: ReactNode }) => (
      <SeccionActivaProvider value={inicial.activa}>{children}</SeccionActivaProvider>
    ),
  });
}

/** `renderHook` no pasa props al wrapper: se cambia la sección activa
 * re-renderizando con un wrapper nuevo. */
function montarConActiva(senalInicial: number) {
  let activa = true;
  const resultado = renderHook(({ senal }: { senal: number }) => useRecargaVisible(senal), {
    initialProps: { senal: senalInicial },
    wrapper: ({ children }: { children: ReactNode }) => (
      <SeccionActivaProvider value={activa}>{children}</SeccionActivaProvider>
    ),
  });
  return {
    ...resultado,
    mostrar(valor: boolean, senal: number) {
      activa = valor;
      resultado.rerender({ senal });
    },
  };
}

describe("useRecargaVisible", () => {
  it("a la vista, sube la versión cada vez que cambia la señal", () => {
    const { result, rerender } = montar({ senal: 0, activa: true });
    expect(result.current).toBe(0);
    rerender({ senal: 1, activa: true });
    expect(result.current).toBe(1);
    rerender({ senal: 2, activa: true });
    expect(result.current).toBe(2);
  });

  it("oculta, la señal no recarga: sube UNA vez al volver a mostrarse", () => {
    const vista = montarConActiva(0);
    vista.mostrar(false, 0);
    vista.mostrar(false, 1);
    vista.mostrar(false, 2);
    expect(vista.result.current).toBe(0);
    vista.mostrar(true, 2);
    expect(vista.result.current).toBe(1);
  });

  it("oculta, un cambio local o una sincronización la deja pendiente", () => {
    const vista = montarConActiva(0);
    vista.mostrar(false, 0);
    act(() => {
      window.dispatchEvent(new Event(EVENTO_CAMBIO_LOCAL_NUBE));
      window.dispatchEvent(new Event(EVENTO_NUBE_ACTUALIZADA));
    });
    expect(vista.result.current).toBe(0);
    vista.mostrar(true, 0);
    expect(vista.result.current).toBe(1);
  });

  it("al volver sin nada pendiente, no recarga", () => {
    const vista = montarConActiva(0);
    vista.mostrar(false, 0);
    vista.mostrar(true, 0);
    expect(vista.result.current).toBe(0);
  });

  it("a la vista, los eventos no la tocan (cada pantalla escucha los suyos)", () => {
    const { result } = montar({ senal: 0, activa: true });
    act(() => {
      window.dispatchEvent(new Event(EVENTO_CAMBIO_LOCAL_NUBE));
    });
    expect(result.current).toBe(0);
  });
});
