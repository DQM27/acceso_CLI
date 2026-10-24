import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Fila } from "./calculo";
import type { Lienzo } from "./esquema";
import LienzoVista from "./Lienzo";

// La torta falla siempre: así se ve qué pasa con un panel roto.
vi.mock("./graficos/Torta", () => ({
  default: () => {
    throw new Error("falla de prueba");
  },
}));

const lienzo: Lienzo = {
  fuente: "contratistas",
  filtros: [],
  widgets: [
    { tipo: "kpi", titulo: "Contratistas", medida: "contar" },
    { tipo: "torta", titulo: "Por estado", categoria: "estado_praind", medida: "contar" },
  ],
};
const filas: Fila[] = [{ nombre: "Ana", estado_praind: "VIGENTE" }];

describe("PanelProtegido", () => {
  it("un panel que falla muestra su aviso y el resto de la vista sigue", () => {
    const silencio = vi.spyOn(console, "error").mockImplementation(() => {});
    render(<LienzoVista lienzo={lienzo} filas={filas} />);
    silencio.mockRestore();
    const aviso = screen.getByRole("alert");
    expect(aviso.textContent).toContain("Por estado");
    expect(aviso.textContent).toContain("No se pudo dibujar este panel");
    expect(aviso.textContent).toContain("falla de prueba");
    // El KPI, al lado, se dibujó igual.
    expect(screen.getByText("Contratistas")).toBeTruthy();
  });
});
