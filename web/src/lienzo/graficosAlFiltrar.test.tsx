import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { Fila } from "./calculo";
import { validarLienzo, type Lienzo } from "./esquema";
import LienzoVista from "./Lienzo";

const vencimientos: Lienzo = {
  fuente: "contratistas",
  filtros: [],
  widgets: [{ tipo: "barras", titulo: "Vencimientos por mes", eje_x: "vencimiento", agrupar_fecha: "mes", medida: "contar" }],
};

const filas: Fila[] = [
  { nombre: "Cris", vencimiento: "2026-11-03" },
  { nombre: "Dora", vencimiento: "2027-01-03" },
  { nombre: "Ana", vencimiento: "2027-05-03" },
  { nombre: "Beto", vencimiento: "2027-05-10" },
];

it("filtrar por la última barra resaltada no rompe el gráfico que queda con menos barras", () => {
  const onEditar = vi.fn();
  const { rerender } = render(<LienzoVista lienzo={vencimientos} filas={filas} onEditar={onEditar} />);
  const grafico = screen.getByRole("img", { name: /Vencimientos por mes/ });
  // Mayo, la última barra, resaltada y elegida.
  fireEvent.focus(grafico);
  fireEvent.keyDown(grafico, { key: "End" });
  fireEvent.keyDown(grafico, { key: "Enter" });
  expect(onEditar).toHaveBeenCalledTimes(1);
  const r = validarLienzo(onEditar.mock.calls[0][0]);
  if (!r.ok) throw new Error(r.errores.join("; "));
  expect(r.lienzo.filtros).toEqual([{ campo: "vencimiento", op: "entre", valor: ["2027-05-01", "2027-05-31"] }]);
  // Antes, el resaltado seguía apuntando a una barra que ya no existía: "Cannot read properties of undefined".
  rerender(<LienzoVista lienzo={r.lienzo} filas={filas.slice(2)} onEditar={onEditar} />);
  expect(screen.getByRole("img", { name: /Vencimientos por mes/ })).toBeTruthy();
});
