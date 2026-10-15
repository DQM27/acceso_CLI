import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import AvisoPermanencia from "./AvisoPermanencia";

describe("AvisoPermanencia", () => {
  it("muestra el texto que le dan", () => {
    render(<AvisoPermanencia texto="2 llevan más de 12 h adentro" activo={false} onAlternar={() => {}} />);
    expect(screen.getByText("2 llevan más de 12 h adentro")).toBeTruthy();
  });

  it("al pulsarlo pide alternar el filtro", () => {
    const onAlternar = vi.fn();
    render(<AvisoPermanencia texto="x" activo={false} onAlternar={onAlternar} />);
    fireEvent.click(screen.getByRole("button"));
    expect(onAlternar).toHaveBeenCalledTimes(1);
  });

  it("refleja si el filtro está puesto", () => {
    const { rerender } = render(<AvisoPermanencia texto="x" activo={false} onAlternar={() => {}} />);
    expect(screen.getByRole("button").getAttribute("aria-pressed")).toBe("false");
    rerender(<AvisoPermanencia texto="x" activo onAlternar={() => {}} />);
    expect(screen.getByRole("button").getAttribute("aria-pressed")).toBe("true");
  });
});
