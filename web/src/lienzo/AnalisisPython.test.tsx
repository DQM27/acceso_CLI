import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Fila } from "./calculo";
import { validarLienzo } from "./esquema";
import LienzoVista from "./Lienzo";

const motor = vi.hoisted(() => ({
  calcularConPython: vi.fn(),
  estado: "listo" as string,
}));

vi.mock("./motorPython", async (original) => ({
  ...(await original<typeof import("./motorPython")>()),
  calcularConPython: motor.calcularConPython,
  escucharEstadoPython: (oyente: (e: string) => void) => {
    oyente(motor.estado);
    return () => {};
  },
}));

const ingresos: Fila[] = [
  { dia: "2026-10-20", nombre: "Ana", empresa: "ACME", tipo_ingreso: "PRAIND", sitio: "Brisas" },
  { dia: "2026-10-21", nombre: "Beto", empresa: "ACME", tipo_ingreso: "PRAIND", sitio: "Brisas" },
];

function conAnalisis(mostrar: Record<string, unknown>) {
  const r = validarLienzo({
    fuente: "ingresos",
    filtros: [],
    widgets: [
      {
        tipo: "analisis",
        titulo: "Pronóstico de ingresos",
        descripcion: "Promedio por día de la semana",
        codigo: "resultado = []",
        columnas: [
          { nombre: "dia", tipo: "fecha", etiqueta: "Día" },
          { nombre: "esperado", tipo: "numero", etiqueta: "Esperados" },
        ],
        mostrar,
      },
    ],
  });
  if (!r.ok) throw new Error(r.errores.join("; "));
  return r.lienzo;
}

beforeEach(() => {
  motor.calcularConPython.mockReset();
  motor.estado = "listo";
});

describe("panel de análisis en Python", () => {
  it("manda el código, las filas del lienzo y las columnas a Python, y dibuja el resultado con el panel común", async () => {
    motor.calcularConPython.mockResolvedValue([
      { dia: "2026-10-23", esperado: "7" },
      { dia: "2026-10-24", esperado: 5 },
    ]);
    render(<LienzoVista lienzo={conAnalisis({ tipo: "tabla", columnas: ["dia", "esperado"] })} filas={ingresos} />);

    expect(screen.getByText("Calculando…")).toBeTruthy();
    expect(motor.calcularConPython).toHaveBeenCalledWith(
      expect.objectContaining({
        codigo: "resultado = []",
        datos: ingresos,
        salida: ["dia", "esperado"],
        columnas: expect.arrayContaining([{ nombre: "dia", tipo: "fecha", etiqueta: "Día" }]),
      }),
    );
    // El título del análisis va en el panel; el "7" que vino como texto se muestra como número.
    expect(await screen.findByText("Pronóstico de ingresos")).toBeTruthy();
    expect(await screen.findByText("23/10/2026")).toBeTruthy();
    expect(screen.getByText("7")).toBeTruthy();
  });

  it("un KPI que suma el resultado", async () => {
    motor.calcularConPython.mockResolvedValue([{ dia: "2026-10-23", esperado: 7 }, { dia: "2026-10-24", esperado: 5 }]);
    render(<LienzoVista lienzo={conAnalisis({ tipo: "kpi", titulo: "x", medida: "sumar", campo: "esperado" })} filas={ingresos} />);
    expect(await screen.findByText("12")).toBeTruthy();
    expect(screen.getByText("Pronóstico de ingresos")).toBeTruthy();
  });

  it("la primera vez avisa que Python se está preparando", () => {
    motor.estado = "cargando";
    motor.calcularConPython.mockReturnValue(new Promise(() => {}));
    render(<LienzoVista lienzo={conAnalisis({ tipo: "tabla", columnas: ["dia"] })} filas={ingresos} />);
    expect(screen.getByText(/Preparando Python/)).toBeTruthy();
  });

  it("si el análisis falla, muestra el motivo en vez del panel", async () => {
    motor.calcularConPython.mockRejectedValue(new Error("ZeroDivisionError (línea 2): division by zero"));
    render(<LienzoVista lienzo={conAnalisis({ tipo: "tabla", columnas: ["dia"] })} filas={ingresos} />);
    expect(await screen.findByText(/El análisis falló: ZeroDivisionError \(línea 2\)/)).toBeTruthy();
  });
});
