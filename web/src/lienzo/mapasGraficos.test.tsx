import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Fila } from "./calculo";
import { validarLienzo, type Lienzo } from "./esquema";
import LienzoVista from "./Lienzo";

function lienzo(documento: unknown): Lienzo {
  const r = validarLienzo(documento);
  if (!r.ok) throw new Error(r.errores.join("; "));
  return r.lienzo;
}

const ingresos: Fila[] = [
  { dia: "2026-10-19", dia_semana: "Lunes", hora: 7, empresa: "A", nombre: "Ana", tipo_ingreso: "PRAIND", sitio: "Brisas" },
  { dia: "2026-10-19", dia_semana: "Lunes", hora: 7, empresa: "A", nombre: "Beto", tipo_ingreso: "PRAIND", sitio: "Brisas" },
  { dia: "2026-10-23", dia_semana: "Viernes", hora: 8, empresa: "B", nombre: "Cris", tipo_ingreso: "SWAT", sitio: "Brisas" },
];

const horasPico = lienzo({
  fuente: "ingresos",
  filtros: [],
  widgets: [{ tipo: "calor", titulo: "Horas pico", filas: "dia_semana", columnas: "hora", medida: "contar" }],
});

describe("mapa de calor", () => {
  it("dibuja una celda por día y hora, con los números, la leyenda de niveles y la tabla accesible", () => {
    const { container } = render(<LienzoVista lienzo={horasPico} filas={ingresos} />);
    expect(screen.getByText("Horas pico")).toBeTruthy();
    // 7 días × 24 horas.
    expect(container.querySelectorAll(".lienzo-calor-celda")).toHaveLength(168);
    expect(container.querySelector(".lienzo-calor-n5")).toBeTruthy();
    expect(screen.getByText("0 h")).toBeTruthy();
    expect(screen.getByText("Lunes")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Ver Horas pico como tabla" }));
    expect(screen.getByRole("columnheader", { name: "Día de la semana" })).toBeTruthy();
    expect(screen.getByRole("columnheader", { name: "8 h" })).toBeTruthy();
  });

  it("clic en una celda con valor: la vista filtrada por ese día y esa hora", () => {
    const onEditar = vi.fn();
    const { container } = render(<LienzoVista lienzo={horasPico} filas={ingresos} onEditar={onEditar} />);
    const celdas = container.querySelectorAll(".lienzo-calor-celda");
    // Lunes a las 8: vacía, no hace nada.
    fireEvent.click(celdas[8]);
    expect(onEditar).not.toHaveBeenCalled();
    // Lunes a las 7.
    fireEvent.click(celdas[7]);
    expect(onEditar).toHaveBeenCalledTimes(1);
    expect(onEditar.mock.calls[0][0].filtros).toEqual([
      { campo: "dia_semana", op: "igual", valor: "Lunes" },
      { campo: "hora", op: "igual", valor: 7 },
    ]);
  });
});

describe("calendario", () => {
  const vencimientos = lienzo({
    fuente: "contratistas",
    filtros: [],
    widgets: [{ tipo: "calendario", titulo: "Vencimientos", fecha: "vencimiento", medida: "contar", meses: 2 }],
  });
  const filas: Fila[] = [
    { nombre: "Ana", vencimiento: "2026-11-03" },
    { nombre: "Beto", vencimiento: "2026-11-03" },
    { nombre: "Cris", vencimiento: "2026-12-10" },
    { nombre: "Dora", vencimiento: "2027-03-01" },
  ];

  it("dibuja los meses con sus días, avisa lo que queda afuera y lista los días con valor en la tabla", () => {
    const { container } = render(<LienzoVista lienzo={vencimientos} filas={filas} />);
    const meses = [...container.querySelectorAll(".lienzo-calendario-mes")].map((m) => m.textContent);
    expect(meses).toHaveLength(2);
    expect(meses[0]).toMatch(/noviembre.*2026/);
    expect(meses[1]).toMatch(/diciembre.*2026/);
    // 30 + 31 días.
    expect(container.querySelectorAll(".lienzo-calor-celda")).toHaveLength(61);
    expect(screen.getByText(/1 más después de diciembre/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Ver Vencimientos como tabla" }));
    expect(screen.getAllByRole("row")).toHaveLength(3);
  });

  it("clic en un día con valor: la vista filtrada por ese día", () => {
    const onEditar = vi.fn();
    const { container } = render(<LienzoVista lienzo={vencimientos} filas={filas} onEditar={onEditar} />);
    const celdas = container.querySelectorAll(".lienzo-calor-celda");
    fireEvent.click(celdas[0]); // 1 de noviembre: vacío
    expect(onEditar).not.toHaveBeenCalled();
    fireEvent.click(celdas[2]); // 3 de noviembre
    expect(onEditar.mock.calls[0][0].filtros).toEqual([{ campo: "vencimiento", op: "igual", valor: "2026-11-03" }]);
  });
});

describe("validación de los paneles nuevos", () => {
  const con = (widget: Record<string, unknown>, fuente = "ingresos") => validarLienzo({ fuente, filtros: [], widgets: [widget] }).ok;

  it("mapa de calor: dos campos distintos de la fuente", () => {
    expect(con({ tipo: "calor", filas: "dia_semana", columnas: "hora", medida: "contar" })).toBe(true);
    expect(con({ tipo: "calor", filas: "hora", columnas: "hora", medida: "contar" })).toBe(false);
    expect(con({ tipo: "calor", filas: "dia_semana", columnas: "nada", medida: "contar" })).toBe(false);
  });

  it("calendario: un campo de fecha y hasta 12 meses", () => {
    expect(con({ tipo: "calendario", fecha: "vencimiento", medida: "contar" }, "contratistas")).toBe(true);
    expect(con({ tipo: "calendario", fecha: "empresa", medida: "contar" }, "contratistas")).toBe(false);
    expect(con({ tipo: "calendario", fecha: "vencimiento", medida: "contar", meses: 13 }, "contratistas")).toBe(false);
  });

  it("los filtros de hora y día de la semana se aceptan en ingresos", () => {
    expect(
      validarLienzo({
        fuente: "ingresos",
        filtros: [
          { campo: "hora", op: "entre", valor: [7, 9] },
          { campo: "dia_semana", op: "en", valor: ["Lunes", "Martes"] },
        ],
        widgets: [{ tipo: "kpi", titulo: "x", medida: "contar" }],
      }).ok,
    ).toBe(true);
  });
});
