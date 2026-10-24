import { afterEach, describe, expect, it } from "vitest";
import { ajustarTipos, calcularConPython, escucharEstadoPython, reiniciarMotorPython, type EstadoPython } from "./motorPython";

/** El iframe de Python que abrió el motor (oculto, uno solo por visita). */
const marco = () => document.querySelector<HTMLIFrameElement>("iframe.lienzo-motor-python");

/** Simula un mensaje que llega desde una ventana (el sandbox u otra). */
/** Lo que el motor le manda al iframe (se captura en vez de mandarlo de verdad). */
function capturarEnvios<T = { id: number }>(): T[] {
  const ventana = marco()?.contentWindow;
  if (!ventana) throw new Error("No hay marco de Python");
  const enviados: T[] = [];
  ventana.postMessage = (mensaje: T) => enviados.push(mensaje);
  return enviados;
}

function llega(data: unknown, source: Window | null = marco()?.contentWindow ?? null) {
  window.dispatchEvent(new MessageEvent("message", { data, source }));
}

const pedido = {
  codigo: "resultado = []",
  datos: [{ dia: "2026-10-22" }],
  columnas: [{ nombre: "dia", tipo: "fecha", etiqueta: "Día" }],
  salida: ["dia", "n"],
};

afterEach(() => reiniciarMotorPython());

describe("motorPython", () => {
  it("abre un solo iframe aislado, espera a que esté preparado y reparte cada resultado por id", async () => {
    const primera = calcularConPython(pedido);
    const segunda = calcularConPython({ ...pedido, salida: ["n"] });
    const iframes = document.querySelectorAll("iframe.lienzo-motor-python");
    expect(iframes).toHaveLength(1);
    expect(marco()?.getAttribute("sandbox")).toBe("allow-scripts");
    expect(marco()?.src).toMatch(/python\.html$/);
    const enviados = capturarEnvios<{ id: number; tipo: string; codigo: string }>();

    // Hasta que el sandbox avisa que está preparado, no se manda nada.
    expect(enviados).toHaveLength(0);
    llega({ tipo: "preparado" });
    expect(enviados).toHaveLength(2);
    const [a, b] = enviados;
    expect(a).toMatchObject({ tipo: "calcular", codigo: "resultado = []" });

    // Las respuestas pueden llegar en cualquier orden; cada una va a su pedido, recortada a sus columnas.
    llega({ tipo: "resultado", id: b.id, filas: [{ n: 3, otra: "x" }] });
    llega({ tipo: "resultado", id: a.id, filas: [{ dia: "2026-10-22", n: 1, mal: { objeto: true } }] });
    expect(await segunda).toEqual([{ n: 3 }]);
    expect(await primera).toEqual([{ dia: "2026-10-22", n: 1 }]);
  });

  it("un error del análisis rechaza sólo ese pedido, con el motivo", async () => {
    const promesa = calcularConPython(pedido);
    const enviados = capturarEnvios();
    llega({ tipo: "preparado" });
    llega({ tipo: "resultado", id: enviados[0].id, error: "ZeroDivisionError (línea 2): division by zero" });
    await expect(promesa).rejects.toThrow("ZeroDivisionError (línea 2)");
  });

  it("ignora mensajes de otras ventanas", async () => {
    const promesa = calcularConPython(pedido);
    const enviados = capturarEnvios();
    llega({ tipo: "preparado" });
    llega({ tipo: "resultado", id: enviados[0].id, filas: [{ dia: "falso" }] }, window);
    llega({ tipo: "resultado", id: enviados[0].id, filas: [{ dia: "2026-10-23" }] });
    expect(await promesa).toEqual([{ dia: "2026-10-23", n: null }]);
  });

  it("avisa el estado de Python (cargando, listo) a quien escucha", () => {
    const estados: EstadoPython[] = [];
    const dejar = escucharEstadoPython((e) => estados.push(e));
    void calcularConPython(pedido).catch(() => {});
    llega({ tipo: "estado", estado: "listo" });
    llega({ tipo: "estado", estado: "cualquiera" });
    dejar();
    expect(estados).toEqual(["apagado", "cargando", "listo"]);
  });
});

describe("ajustarTipos", () => {
  it("lleva cada valor al tipo declarado; lo que no encaja queda vacío", () => {
    const columnas = [
      { nombre: "dia", tipo: "fecha" },
      { nombre: "n", tipo: "numero" },
      { nombre: "ok", tipo: "booleano" },
      { nombre: "nombre", tipo: "texto" },
    ];
    expect(
      ajustarTipos(
        [
          { dia: "2026-10-22T00:00:00", n: "12", ok: true, nombre: 5 },
          { dia: "ayer", n: "doce", ok: "si", nombre: null },
        ],
        columnas,
      ),
    ).toEqual([
      { dia: "2026-10-22", n: 12, ok: true, nombre: "5" },
      { dia: null, n: null, ok: null, nombre: null },
    ]);
  });
});
