import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import Modal, { DURACION_SALIDA_MODAL_MS } from "./Modal";

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

  it("Escape cierra el modal después de la animación de salida", async () => {
    const onCerrar = vi.fn();
    render(
      <Modal titulo="T" onCerrar={onCerrar}>
        <p>x</p>
      </Modal>,
    );
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onCerrar).not.toHaveBeenCalled();
    await waitFor(() => expect(onCerrar).toHaveBeenCalledOnce());
  });

  it("el botón X cierra el modal después de la animación de salida", async () => {
    const onCerrar = vi.fn();
    render(
      <Modal titulo="T" onCerrar={onCerrar}>
        <p>x</p>
      </Modal>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Cerrar" }));
    expect(onCerrar).not.toHaveBeenCalled();
    // Diálogo con su título como nombre, para los lectores de pantalla.
    expect(screen.getByRole("dialog")).toBeTruthy();
    await waitFor(() => expect(onCerrar).toHaveBeenCalledOnce());
  });

  it("cerrar dos veces (X y Esc) avisa una sola vez", async () => {
    const onCerrar = vi.fn();
    render(
      <Modal titulo="T" onCerrar={onCerrar}>
        <p>x</p>
      </Modal>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Cerrar" }));
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(onCerrar).toHaveBeenCalled());
    await new Promise((resolver) => setTimeout(resolver, DURACION_SALIDA_MODAL_MS + 50));
    expect(onCerrar).toHaveBeenCalledOnce();
  });

  it("si la pantalla no lo desmonta al cerrar, el modal vuelve a verse", async () => {
    const { container } = render(
      <Modal titulo="T" onCerrar={() => {}}>
        <p>x</p>
      </Modal>,
    );
    const velo = container.firstChild as Element;
    fireEvent.keyDown(window, { key: "Escape" });
    expect(velo.classList.contains("modal-velo-saliendo")).toBe(true);
    await waitFor(() => expect(velo.classList.contains("modal-velo-saliendo")).toBe(false));
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
