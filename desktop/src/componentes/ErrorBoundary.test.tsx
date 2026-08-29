import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ErrorBoundary from "./ErrorBoundary";

function ComponenteQueRompe(): never {
  throw new Error("boom de prueba");
}

describe("ErrorBoundary", () => {
  // React (además de nuestro componentDidCatch) también loguea el error a
  // consola por su cuenta — silenciarlo acá evita ruido en la salida del
  // test sin ocultar una falla real (el spy se restaura después de cada
  // test, y las aserciones no dependen de qué se logueó).
  beforeEach(() => {
    vi.spyOn(console, "error").mockImplementation(() => {});
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renderiza a los hijos normalmente cuando no hay error", () => {
    render(
      <ErrorBoundary>
        <p>Todo bien</p>
      </ErrorBoundary>,
    );
    expect(screen.getByText("Todo bien")).toBeTruthy();
  });

  it("muestra la pantalla de error en vez de tumbar toda la app", () => {
    render(
      <ErrorBoundary>
        <ComponenteQueRompe />
      </ErrorBoundary>,
    );
    expect(screen.getByText("Ocurrió un error inesperado")).toBeTruthy();
    expect(screen.getByText("boom de prueba")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Reiniciar" })).toBeTruthy();
  });
});
