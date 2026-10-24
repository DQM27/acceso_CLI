import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import PanelLibre from "./PanelLibre";
import { URL_SANDBOX } from "./sandbox";
import type { WidgetLibre } from "./esquema";

const WIDGET: WidgetLibre = {
  tipo: "libre",
  titulo: "Mapa de calor",
  descripcion: "Ingresos por empresa y día",
  codigo: "return h.svg(ancho, 10, '')",
};
const FILAS = [{ empresa: "ALDAMA", dia: "2026-10-20" }];

function montar() {
  render(<PanelLibre widget={WIDGET} filas={FILAS} origen="ingresos" />);
  const marco = screen.getByTitle("Ingresos por empresa y día") as HTMLIFrameElement;
  const ventana = marco.contentWindow as Window;
  const enviar = vi.spyOn(ventana, "postMessage");
  const recibir = (data: unknown, source: Window | null = ventana) =>
    act(() => {
      window.dispatchEvent(new MessageEvent("message", { data, source }));
    });
  return { marco, enviar, recibir };
}

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("PanelLibre", () => {
  it("el dibujo va en un iframe aislado del sandbox: sólo scripts, origen anónimo, sin referer", () => {
    const { marco } = montar();
    expect(marco.getAttribute("src")).toBe(URL_SANDBOX);
    expect(marco.getAttribute("sandbox")).toBe("allow-scripts");
    expect(marco.getAttribute("referrerpolicy")).toBe("no-referrer");
    expect(screen.getByText("Mapa de calor")).toBeTruthy();
  });

  it("manda código, filas y columnas recién cuando el sandbox avisa que está listo", () => {
    const { enviar, recibir } = montar();
    expect(enviar).not.toHaveBeenCalled();
    recibir({ tipo: "preparado" });
    expect(enviar).toHaveBeenCalledTimes(1);
    const [pedido] = enviar.mock.calls[0] as [Record<string, unknown>];
    expect(pedido.tipo).toBe("dibujar");
    expect(pedido.codigo).toBe(WIDGET.codigo);
    expect(pedido.datos).toEqual(FILAS);
    expect(pedido.columnas).toContainEqual({ nombre: "empresa", tipo: "texto", etiqueta: "Empresa" });
  });

  it("ignora mensajes de cualquier otra ventana", () => {
    const { enviar, recibir } = montar();
    recibir({ tipo: "preparado" }, window);
    recibir({ tipo: "error", mensaje: "falso" }, window);
    expect(enviar).not.toHaveBeenCalled();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("ajusta el alto que pide el sandbox (con límites) y muestra sus errores", () => {
    const { marco, recibir } = montar();
    recibir({ tipo: "alto", alto: 420 });
    expect(marco.style.height).toBe("420px");
    recibir({ tipo: "alto", alto: 99999 });
    expect(marco.style.height).toBe("1600px");
    recibir({ tipo: "error", mensaje: "h.svg is not a function" });
    expect(screen.getByRole("alert").textContent).toContain("h.svg is not a function");
    recibir({ tipo: "listo" });
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("si el sandbox no carga, lo dice en vez de dejar un hueco", () => {
    vi.useFakeTimers();
    render(<PanelLibre widget={WIDGET} filas={FILAS} origen="ingresos" />);
    act(() => {
      vi.advanceTimersByTime(10_500);
    });
    expect(screen.getByRole("alert").textContent).toContain("No se pudo cargar el espacio de dibujo");
  });
});
