import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import Modal from "./Modal";

describe("Modal", () => {
  it("muestra título y contenido", () => {
    render(
      <Modal titulo="Un título" onCerrar={() => {}}>
        <p>contenido</p>
      </Modal>,
    );
    expect(screen.getByText("Un título")).toBeTruthy();
    expect(screen.getByText("contenido")).toBeTruthy();
  });

  it("Escape cierra el modal", () => {
    const onCerrar = vi.fn();
    render(
      <Modal titulo="T" onCerrar={onCerrar}>
        <p>x</p>
      </Modal>,
    );
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onCerrar).toHaveBeenCalledOnce();
  });

  it("el botón X cierra el modal", () => {
    const onCerrar = vi.fn();
    render(
      <Modal titulo="T" onCerrar={onCerrar}>
        <p>x</p>
      </Modal>,
    );
    fireEvent.click(screen.getByRole("button", { name: "✕" }));
    expect(onCerrar).toHaveBeenCalledOnce();
  });

  it("click en el backdrop NO cierra -- decisión explícita, no accidental", () => {
    const onCerrar = vi.fn();
    const { container } = render(
      <Modal titulo="T" onCerrar={onCerrar}>
        <p>x</p>
      </Modal>,
    );
    // El backdrop es el div raíz que envuelve todo -- clickearlo a propósito,
    // no la tarjeta ni su contenido.
    fireEvent.click(container.firstChild as Element);
    expect(onCerrar).not.toHaveBeenCalled();
  });
});
