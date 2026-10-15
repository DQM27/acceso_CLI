import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import VersionFooter from "./VersionFooter";

describe("VersionFooter", () => {
  it("muestra la versión real de Tauri (no la de package.json)", async () => {
    render(<VersionFooter onBuscarActualizacion={() => {}} />);
    await waitFor(() => expect(screen.getByText("v0.0.0-test")).toBeTruthy());
  });

  it("conserva el title con el nombre completo para el tooltip", async () => {
    const { container } = render(<VersionFooter onBuscarActualizacion={() => {}} />);
    await waitFor(() =>
      expect(container.querySelector(".shell-sidebar-version")?.getAttribute("title")).toBe(
        "Lattis v0.0.0-test — buscar actualización",
      ),
    );
  });

  it("no renderiza nada mientras la versión todavía no llegó", () => {
    const { container } = render(<VersionFooter onBuscarActualizacion={() => {}} />);
    expect(container.querySelector(".shell-sidebar-version")).toBeNull();
  });

  it("al hacer clic pide buscar actualización", async () => {
    const onBuscar = vi.fn();
    render(<VersionFooter onBuscarActualizacion={onBuscar} />);
    fireEvent.click(await screen.findByText("v0.0.0-test"));
    expect(onBuscar).toHaveBeenCalledTimes(1);
  });

  it("mientras busca, lo dice y no deja pulsar de nuevo", async () => {
    const onBuscar = vi.fn();
    const { container } = render(<VersionFooter onBuscarActualizacion={onBuscar} buscando />);
    await waitFor(() => expect(screen.getByText("Buscando…")).toBeTruthy());
    const boton = container.querySelector(".shell-sidebar-version") as HTMLButtonElement;
    expect(boton.disabled).toBe(true);
  });
});
