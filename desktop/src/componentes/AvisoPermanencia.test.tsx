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

  it("el nivel crítico va en rojo y con el ícono parpadeando; el aviso no", () => {
    const { rerender, container } = render(
      <AvisoPermanencia texto="1 lleva más de 14 h adentro" activo={false} critico onAlternar={() => {}} />,
    );
    expect(screen.getByRole("button").className).toContain("aviso-permanencia-critico");
    expect(container.querySelector("svg")?.getAttribute("class")).toContain("punto-conexion-parpadeo");
    rerender(<AvisoPermanencia texto="x" activo={false} onAlternar={() => {}} />);
    expect(screen.getByRole("button").className).not.toContain("aviso-permanencia-critico");
    expect(container.querySelector("svg")?.getAttribute("class") ?? "").not.toContain(
      "punto-conexion-parpadeo",
    );
  });
});
