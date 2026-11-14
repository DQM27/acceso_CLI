import { act, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useListaFlotante } from "./ListaFlotante.logica";

describe("useListaFlotante", () => {
  function Envoltorio({ visible }: { visible: boolean }) {
    const { campoRef, posicion } = useListaFlotante(visible);
    return (
      <div>
        <div ref={campoRef} data-testid="campo" />
        <span data-testid="posicion">{posicion ? "visible" : "oculto"}</span>
      </div>
    );
  }

  it("sin visible, no calcula posición", () => {
    render(<Envoltorio visible={false} />);
    expect(screen.getByTestId("posicion").textContent).toBe("oculto");
  });

  it("visible con el campo ya montado, calcula una posición", () => {
    render(<Envoltorio visible={true} />);
    expect(screen.getByTestId("posicion").textContent).toBe("visible");
  });

  it("sigue al campo si se mueve mientras la lista está visible", async () => {
    // Reproduce el bug de 2026-09-24: el modal se encoge animado y el
    // buscador se desplaza; la lista tiene que acompañarlo, no quedarse
    // donde lo midió la primera vez.
    function EnvoltorioTop() {
      const { campoRef, posicion } = useListaFlotante(true);
      return (
        <div>
          <div ref={campoRef} data-testid="campo" />
          <span data-testid="top">{posicion?.top ?? "sin"}</span>
        </div>
      );
    }
    let bottom = 100;
    const espia = vi
      .spyOn(HTMLElement.prototype, "getBoundingClientRect")
      .mockImplementation(
        () => ({ top: bottom - 40, bottom, left: 10, right: 310, width: 300 }) as DOMRect,
      );
    render(<EnvoltorioTop />);
    expect(screen.getByTestId("top").textContent).toBe("104");

    bottom = 250;
    await act(() => new Promise((listo) => requestAnimationFrame(() => listo(undefined))));
    await act(() => new Promise((listo) => requestAnimationFrame(() => listo(undefined))));
    expect(screen.getByTestId("top").textContent).toBe("254");
    espia.mockRestore();
  });

  it("al dejar de estar visible, vuelve a null", () => {
    const { rerender } = render(<Envoltorio visible={true} />);
    expect(screen.getByTestId("posicion").textContent).toBe("visible");

    rerender(<Envoltorio visible={false} />);
    expect(screen.getByTestId("posicion").textContent).toBe("oculto");
  });
});
