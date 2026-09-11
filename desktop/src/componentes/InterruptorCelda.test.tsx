import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { CustomCellRendererProps } from "ag-grid-react";
import InterruptorCelda from "./InterruptorCelda";

/** Sólo `value`/`setValue`/`critico` importan acá -- el resto de
 * `CustomCellRendererProps` (nodo/API de AG Grid) no lo toca este
 * componente, así que el cast evita armar un mock gigante sin valor real. */
function props(
  overrides: Partial<CustomCellRendererProps<unknown, boolean> & { critico?: boolean }> = {},
) {
  return {
    value: false,
    setValue: () => {},
    ...overrides,
  } as CustomCellRendererProps<unknown, boolean> & { critico?: boolean };
}

describe("InterruptorCelda", () => {
  it("refleja el valor en aria-checked", () => {
    render(<InterruptorCelda {...props({ value: true })} />);
    expect(screen.getByRole("switch").getAttribute("aria-checked")).toBe("true");
  });

  it("al hacer click, llama a setValue con el valor invertido", () => {
    const setValue = vi.fn();
    render(<InterruptorCelda {...props({ value: false, setValue })} />);
    fireEvent.click(screen.getByRole("switch"));
    expect(setValue).toHaveBeenCalledWith(true);
  });

  it("critico agrega la clase interruptor-critico", () => {
    render(<InterruptorCelda {...props({ value: true, critico: true })} />);
    expect(screen.getByRole("switch").className).toContain("interruptor-critico");
  });

  it("sin critico, no agrega esa clase", () => {
    render(<InterruptorCelda {...props({ value: true })} />);
    expect(screen.getByRole("switch").className).not.toContain("interruptor-critico");
  });
});
