import { act, fireEvent, render, renderHook, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import BarraSeleccion from "./BarraSeleccion";
import { usePresencia } from "./usePresencia";

const etiqueta = { singular: "seleccionado", plural: "seleccionados" };

function montar(cantidad: number, acciones: { onAccion?: () => void; onLimpiar?: () => void } = {}) {
  return render(
    <BarraSeleccion
      cantidad={cantidad}
      etiqueta={etiqueta}
      accion="Registrar salida"
      onAccion={acciones.onAccion ?? (() => {})}
      onLimpiar={acciones.onLimpiar ?? (() => {})}
    />,
  );
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("usePresencia", () => {
  it("activo: montado y sin salir", () => {
    const { result } = renderHook(() => usePresencia(true, 100));
    expect(result.current).toEqual({ montado: true, saliendo: false });
  });

  it("al desactivarse sigue montado saliendo y se desmonta tras la duración", () => {
    const { result, rerender } = renderHook(({ activo }) => usePresencia(activo, 100), {
      initialProps: { activo: true },
    });
    rerender({ activo: false });
    expect(result.current).toEqual({ montado: true, saliendo: true });
    act(() => {
      vi.advanceTimersByTime(100);
    });
    expect(result.current).toEqual({ montado: false, saliendo: false });
  });

  it("si vuelve a activarse durante la salida, se queda montado", () => {
    const { result, rerender } = renderHook(({ activo }) => usePresencia(activo, 100), {
      initialProps: { activo: true },
    });
    rerender({ activo: false });
    rerender({ activo: true });
    act(() => {
      vi.advanceTimersByTime(500);
    });
    expect(result.current).toEqual({ montado: true, saliendo: false });
  });
});

describe("BarraSeleccion", () => {
  it("sin selección no se ve nada", () => {
    montar(0);
    expect(screen.queryByRole("region", { name: "Acciones para la selección" })).toBeNull();
  });

  it("con selección dice cuántos hay, en plural", () => {
    montar(3);
    expect(screen.getByText("3 seleccionados")).toBeTruthy();
  });

  it("con uno solo lo dice en singular", () => {
    montar(1);
    expect(screen.getByText("1 seleccionado")).toBeTruthy();
  });

  it("una acción crítica va en rojo y una normal en el acento", () => {
    const { rerender } = render(
      <BarraSeleccion
        cantidad={2}
        etiqueta={etiqueta}
        accion="Registrar salida"
        critica
        onAccion={() => {}}
        onLimpiar={() => {}}
      />,
    );
    expect(screen.getByText("Registrar salida").className).toContain("boton-peligro-solido");
    rerender(
      <BarraSeleccion
        cantidad={2}
        etiqueta={etiqueta}
        accion="Registrar salida"
        onAccion={() => {}}
        onLimpiar={() => {}}
      />,
    );
    expect(screen.getByText("Registrar salida").className).toContain("boton-primario");
  });

  it("el botón de acción y la ✕ llaman a sus callbacks", () => {
    const onAccion = vi.fn();
    const onLimpiar = vi.fn();
    montar(2, { onAccion, onLimpiar });
    fireEvent.click(screen.getByText("Registrar salida"));
    fireEvent.click(screen.getByLabelText("Quitar la selección"));
    expect(onAccion).toHaveBeenCalledTimes(1);
    expect(onLimpiar).toHaveBeenCalledTimes(1);
  });

  it("al quedar sin selección sale: conserva el último número mientras se desvanece y luego desaparece", () => {
    const { rerender } = montar(3);
    rerender(
      <BarraSeleccion
        cantidad={0}
        etiqueta={etiqueta}
        accion="Registrar salida"
        onAccion={() => {}}
        onLimpiar={() => {}}
      />,
    );
    expect(screen.getByText("3 seleccionados")).toBeTruthy();
    expect(
      screen.getByRole("region", { name: "Acciones para la selección" }).className,
    ).toContain("barra-seleccion-saliendo");
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(screen.queryByRole("region", { name: "Acciones para la selección" })).toBeNull();
  });
});
