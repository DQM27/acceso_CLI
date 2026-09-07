import { act, render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { BarraEstadoProvider, SeccionActivaProvider, useBarraEstado } from "./BarraEstadoContexto";

/**
 * Con App.tsx montando todas las secciones visitadas a la vez (ocultas con
 * CSS, ya no desmontadas -- ver el doc-comment de `visitadas` en
 * Shell/App.tsx), `useBarraEstado` ya no puede confiar en "me
 * desmontaron" para saber cuándo dejar de estar activa -- de ahí
 * `SeccionActivaContexto`. Este test prueba justo el bug real que motivó
 * el cambio: sin él, el mensaje de una sección vieja se quedaba pegado en
 * la barra de estado al cambiar a otra que ya estaba montada de antes.
 */
function Publicador({ mensaje }: { mensaje: string }) {
  useBarraEstado(mensaje);
  return null;
}

describe("useBarraEstado + SeccionActivaContexto", () => {
  it("publica el mensaje mientras la sección está activa", () => {
    const establecer = vi.fn();
    render(
      <BarraEstadoProvider value={establecer}>
        <SeccionActivaProvider value={true}>
          <Publicador mensaje="hola" />
        </SeccionActivaProvider>
      </BarraEstadoProvider>,
    );
    expect(establecer).toHaveBeenCalledWith("hola");
  });

  it("no publica nada mientras la sección NO está activa", () => {
    const establecer = vi.fn();
    render(
      <BarraEstadoProvider value={establecer}>
        <SeccionActivaProvider value={false}>
          <Publicador mensaje="hola" />
        </SeccionActivaProvider>
      </BarraEstadoProvider>,
    );
    expect(establecer).not.toHaveBeenCalledWith("hola");
  });

  it("limpia el mensaje al dejar de estar activa (sin desmontarse) -- el bug real que esto arregla", () => {
    const establecer = vi.fn();
    function Arnes({ activa }: { activa: boolean }) {
      return (
        <BarraEstadoProvider value={establecer}>
          <SeccionActivaProvider value={activa}>
            <Publicador mensaje="hola" />
          </SeccionActivaProvider>
        </BarraEstadoProvider>
      );
    }

    const { rerender } = render(<Arnes activa={true} />);
    establecer.mockClear();

    // Se vuelve inactiva SIN desmontarse (mismo componente, misma
    // identidad -- exactamente lo que pasa ahora al cambiar de sección
    // con las pantallas montadas para siempre).
    act(() => {
      rerender(<Arnes activa={false} />);
    });
    expect(establecer).toHaveBeenCalledWith(null);
  });

  it("vuelve a publicar el mensaje al reactivarse", () => {
    const establecer = vi.fn();
    function Arnes({ activa }: { activa: boolean }) {
      return (
        <BarraEstadoProvider value={establecer}>
          <SeccionActivaProvider value={activa}>
            <Publicador mensaje="hola de nuevo" />
          </SeccionActivaProvider>
        </BarraEstadoProvider>
      );
    }

    const { rerender } = render(<Arnes activa={true} />);
    act(() => rerender(<Arnes activa={false} />));
    establecer.mockClear();

    act(() => rerender(<Arnes activa={true} />));
    expect(establecer).toHaveBeenCalledWith("hola de nuevo");
  });
});
