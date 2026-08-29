import { act, render, renderHook, screen } from "@testing-library/react";
import type { KeyboardEvent } from "react";
import { describe, expect, it, vi } from "vitest";
import { useListaFlotante, useNavegacionFlechas } from "./ListaFlotante";

/** Evento de teclado mínimo — `manejarTecla` sólo lee `.key` y llama
 * `.preventDefault()`, no hace falta un evento real de DOM para probarlo. */
function tecla(key: string) {
  return { key, preventDefault: vi.fn() } as unknown as KeyboardEvent<HTMLInputElement>;
}

describe("useNavegacionFlechas", () => {
  // Referencias estables a propósito, declaradas una sola vez: el hook
  // reinicia `resaltado` cuando cambia la IDENTIDAD de `items` (efecto
  // sobre `[items]`), igual que en NuevoIngresoModal/SalidaModal donde
  // viene de useState/useMemo. Pasar un literal nuevo en cada render del
  // hook (p. ej. `() => useNavegacionFlechas(["a","b"], ...)` dentro de
  // `renderHook`) dispararía ese reinicio en cada tecla — no es cómo se usa
  // en la app real, así que tampoco es cómo hay que probarlo acá.
  const items3 = ["a", "b", "c"];
  const items2 = ["a", "b"];
  const items1 = ["a"];

  it("arranca resaltando el primer ítem", () => {
    const { result } = renderHook(() => useNavegacionFlechas(items3, true, vi.fn()));
    expect(result.current.resaltado).toBe(0);
  });

  it("ArrowDown avanza y ArrowUp retrocede, sin salirse de la lista", () => {
    const { result } = renderHook(() => useNavegacionFlechas(items3, true, vi.fn()));

    act(() => result.current.manejarTecla(tecla("ArrowDown")));
    expect(result.current.resaltado).toBe(1);

    act(() => result.current.manejarTecla(tecla("ArrowDown")));
    act(() => result.current.manejarTecla(tecla("ArrowDown")));
    // Ya en el último (índice 2) — otro ArrowDown no debe pasarse de largo.
    expect(result.current.resaltado).toBe(2);

    act(() => result.current.manejarTecla(tecla("ArrowUp")));
    expect(result.current.resaltado).toBe(1);
  });

  it("ArrowUp en el primero se queda en 0, no baja de rango", () => {
    const { result } = renderHook(() => useNavegacionFlechas(items2, true, vi.fn()));
    act(() => result.current.manejarTecla(tecla("ArrowUp")));
    expect(result.current.resaltado).toBe(0);
  });

  it("Enter llama a onSeleccionar con el ítem resaltado", () => {
    const onSeleccionar = vi.fn();
    const { result } = renderHook(() => useNavegacionFlechas(items3, true, onSeleccionar));

    act(() => result.current.manejarTecla(tecla("ArrowDown")));
    act(() => result.current.manejarTecla(tecla("Enter")));

    expect(onSeleccionar).toHaveBeenCalledTimes(1);
    expect(onSeleccionar).toHaveBeenCalledWith("b");
  });

  it("previene el comportamiento por defecto en las teclas que maneja", () => {
    const { result } = renderHook(() => useNavegacionFlechas(items1, true, vi.fn()));
    const evento = tecla("ArrowDown");
    act(() => result.current.manejarTecla(evento));
    expect(evento.preventDefault).toHaveBeenCalledTimes(1);
  });

  it("no hace nada si la lista no está activa (p. ej. oculta)", () => {
    const onSeleccionar = vi.fn();
    const { result } = renderHook(() => useNavegacionFlechas(items2, false, onSeleccionar));
    const evento = tecla("Enter");
    act(() => result.current.manejarTecla(evento));
    expect(onSeleccionar).not.toHaveBeenCalled();
    expect(evento.preventDefault).not.toHaveBeenCalled();
  });

  it("no hace nada con la lista vacía", () => {
    const onSeleccionar = vi.fn();
    const { result } = renderHook(() => useNavegacionFlechas([], true, onSeleccionar));
    act(() => result.current.manejarTecla(tecla("Enter")));
    expect(onSeleccionar).not.toHaveBeenCalled();
  });

  it("reinicia el resaltado a 0 cuando cambia la lista de ítems", () => {
    const { result, rerender } = renderHook(
      ({ items }: { items: string[] }) => useNavegacionFlechas(items, true, vi.fn()),
      { initialProps: { items: ["a", "b", "c"] } },
    );

    act(() => result.current.manejarTecla(tecla("ArrowDown")));
    expect(result.current.resaltado).toBe(1);

    // Una búsqueda nueva con menos resultados — si no se reiniciara, el
    // resaltado podría apuntar a un índice que ya no existe.
    rerender({ items: ["x"] });
    expect(result.current.resaltado).toBe(0);
  });

  it("otras teclas no mueven el resaltado ni seleccionan", () => {
    const onSeleccionar = vi.fn();
    const { result } = renderHook(() => useNavegacionFlechas(items2, true, onSeleccionar));
    act(() => result.current.manejarTecla(tecla("a")));
    expect(result.current.resaltado).toBe(0);
    expect(onSeleccionar).not.toHaveBeenCalled();
  });
});

describe("useListaFlotante", () => {
  function Envoltorio({ visible }: { visible: boolean }) {
    const { campoRef, posicion } = useListaFlotante(visible);
    return (
      <div>
        <div ref={campoRef} data-testid="campo" />
        <span data-testid="posicion">{posicion ? "visible" : "oculto"}</span>
      </div>
    );
  }

  it("sin visible, no calcula posición", () => {
    render(<Envoltorio visible={false} />);
    expect(screen.getByTestId("posicion").textContent).toBe("oculto");
  });

  it("visible con el campo ya montado, calcula una posición", () => {
    render(<Envoltorio visible={true} />);
    expect(screen.getByTestId("posicion").textContent).toBe("visible");
  });

  it("al dejar de estar visible, vuelve a null", () => {
    const { rerender } = render(<Envoltorio visible={true} />);
    expect(screen.getByTestId("posicion").textContent).toBe("visible");

    rerender(<Envoltorio visible={false} />);
    expect(screen.getByTestId("posicion").textContent).toBe("oculto");
  });
});
