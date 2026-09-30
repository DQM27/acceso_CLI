import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import Paginador, { textoRangoPagina, totalPaginas } from "./Paginador";

describe("textoRangoPagina / totalPaginas", () => {
  it("arma el rango de la página y recorta la última", () => {
    expect(textoRangoPagina(0, 100, 4321)).toContain("de");
    expect(textoRangoPagina(0, 100, 4321).startsWith("1–100")).toBe(true);
    expect(textoRangoPagina(43, 100, 4321).startsWith("4")).toBe(true);
    expect(textoRangoPagina(43, 100, 4321)).toMatch(/4.?301–4.?321 de 4.?321/);
  });

  it("sin filas dice 'Sin resultados' y hay al menos una página", () => {
    expect(textoRangoPagina(0, 100, 0)).toBe("Sin resultados");
    expect(totalPaginas(100, 0)).toBe(1);
    expect(totalPaginas(100, 100)).toBe(1);
    expect(totalPaginas(100, 101)).toBe(2);
  });
});

describe("Paginador", () => {
  it("anterior deshabilitado en la primera página y siguiente en la última", () => {
    const boton = (nombre: string) => screen.getByLabelText(nombre) as HTMLButtonElement;
    const { rerender } = render(<Paginador pagina={0} tamano={100} total={250} onCambiar={() => {}} />);
    expect(boton("Página anterior").disabled).toBe(true);
    expect(boton("Página siguiente").disabled).toBe(false);

    rerender(<Paginador pagina={2} tamano={100} total={250} onCambiar={() => {}} />);
    expect(boton("Página anterior").disabled).toBe(false);
    expect(boton("Página siguiente").disabled).toBe(true);
  });

  it("avisa la página a la que se va", () => {
    const alCambiar = vi.fn();
    render(<Paginador pagina={1} tamano={100} total={250} onCambiar={alCambiar} />);

    fireEvent.click(screen.getByLabelText("Página siguiente"));
    expect(alCambiar).toHaveBeenCalledWith(2);
    fireEvent.click(screen.getByLabelText("Página anterior"));
    expect(alCambiar).toHaveBeenCalledWith(0);
  });
});
