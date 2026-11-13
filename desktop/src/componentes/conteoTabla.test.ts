import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { textoConteo, useConteoVisible } from "./conteoTabla";

describe("textoConteo", () => {
  it("sin conteo todavía (la grilla no informó) muestra el total", () => {
    expect(textoConteo(null, 304, "movimiento(s)")).toBe("304 movimiento(s)");
  });

  it("sin filtros muestra sólo el total", () => {
    expect(textoConteo({ visibles: 304, filtrando: false }, 304, "movimiento(s)")).toBe(
      "304 movimiento(s)",
    );
  });

  it("con un filtro muestra cuántas quedan de cuántas", () => {
    expect(textoConteo({ visibles: 1, filtrando: true }, 304, "movimiento(s)")).toBe(
      "1 de 304 movimiento(s)",
    );
  });

  it("un filtro sin coincidencias dice 0, no el total", () => {
    expect(textoConteo({ visibles: 0, filtrando: true }, 304, "movimiento(s)")).toBe(
      "0 de 304 movimiento(s)",
    );
  });

  it("un filtro que deja todas las filas igual dice que hay filtro", () => {
    expect(textoConteo({ visibles: 20, filtrando: true }, 20, "resultado(s)")).toBe(
      "20 de 20 resultado(s)",
    );
  });

  it("acepta un total de texto (rango truncado)", () => {
    expect(textoConteo({ visibles: 12, filtrando: true }, "500+", "movimiento(s)")).toBe(
      "12 de 500+ movimiento(s)",
    );
  });
});

describe("useConteoVisible", () => {
  it("arranca sin conteo y guarda el que informa la grilla", () => {
    const { result } = renderHook(() => useConteoVisible());
    expect(result.current[0]).toBeNull();
    act(() => result.current[1]({ visibles: 3, filtrando: true }));
    expect(result.current[0]).toEqual({ visibles: 3, filtrando: true });
  });

  it("un conteo igual (ej. sólo cambió el orden) conserva el mismo objeto", () => {
    const { result } = renderHook(() => useConteoVisible());
    act(() => result.current[1]({ visibles: 3, filtrando: true }));
    const anterior = result.current[0];
    act(() => result.current[1]({ visibles: 3, filtrando: true }));
    expect(result.current[0]).toBe(anterior);
  });

  it("el callback es estable entre renders", () => {
    const { result, rerender } = renderHook(() => useConteoVisible());
    const reportar = result.current[1];
    rerender();
    expect(result.current[1]).toBe(reportar);
  });
});
