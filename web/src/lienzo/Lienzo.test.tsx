import { render, screen, within, cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type { ResumenHoy } from "../api/lienzo";
import type { Fila } from "./calculo";
import LienzoVista from "./Lienzo";
import ResumenDeHoy from "./ResumenHoy";
import { validarLienzo } from "./esquema";
import { VISTAS_PREDEFINIDAS } from "./vistas";

afterEach(cleanup);

const filas: Fila[] = [
  { nombre: "Ana", empresa: "ACME", vencimiento: "2026-10-05", dias_para_vencer: -3 },
  { nombre: "Beto", empresa: "ACME", vencimiento: "2026-10-01", dias_para_vencer: -7 },
  { nombre: "Cris", empresa: "OTRA", vencimiento: "2026-09-01", dias_para_vencer: -40 },
];

function lienzo(widgets: unknown[]) {
  const r = validarLienzo({ fuente: "contratistas", filtros: [], widgets });
  if (!r.ok) throw new Error(r.errores.join("; "));
  return r.lienzo;
}

describe("LienzoVista", () => {
  it("dibuja un KPI con su título y valor", () => {
    render(<LienzoVista lienzo={lienzo([{ tipo: "kpi", titulo: "Vencidos", medida: "contar" }])} filas={filas} />);
    expect(screen.getByText("Vencidos")).toBeTruthy();
    expect(screen.getByText("3")).toBeTruthy();
  });

  it("dibuja barras con una barra por categoría y un título accesible", () => {
    const { container } = render(
      <LienzoVista
        lienzo={lienzo([{ tipo: "barras", titulo: "Por empresa", eje_x: "empresa", medida: "contar" }])}
        filas={filas}
      />,
    );
    expect(screen.getByRole("img", { name: "Por empresa" })).toBeTruthy();
    expect(container.querySelectorAll("rect.lienzo-barra")).toHaveLength(2);
  });

  it("dibuja la tabla con encabezados y filas", () => {
    render(
      <LienzoVista
        lienzo={lienzo([{ tipo: "tabla", columnas: ["nombre", "empresa"], orden: { campo: "nombre", dir: "asc" } }])}
        filas={filas}
      />,
    );
    const tabla = screen.getByRole("table");
    expect(within(tabla).getByRole("columnheader", { name: "Empresa" })).toBeTruthy();
    expect(within(tabla).getAllByRole("row")).toHaveLength(4);
  });

  it("sin datos muestra un aviso en vez de un gráfico vacío", () => {
    render(<LienzoVista lienzo={lienzo([{ tipo: "tabla", columnas: ["nombre"] }])} filas={[]} />);
    expect(screen.getByText("Sin datos para mostrar.")).toBeTruthy();
  });

  it("el texto de los datos se muestra como texto, nunca como HTML", () => {
    const peligrosa: Fila[] = [{ nombre: "<img src=x onerror=alert(1)>", empresa: "<b>X</b>" }];
    const { container } = render(
      <LienzoVista lienzo={lienzo([{ tipo: "tabla", columnas: ["nombre", "empresa"] }])} filas={peligrosa} />,
    );
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("b")).toBeNull();
    expect(screen.getByText("<img src=x onerror=alert(1)>")).toBeTruthy();
  });

  it("todas las vistas de ejemplo validan y se dibujan", () => {
    for (const vista of VISTAS_PREDEFINIDAS) {
      const r = validarLienzo(vista.lienzo);
      expect(r.ok, `${vista.id}: ${r.ok ? "" : r.errores.join("; ")}`).toBe(true);
      if (!r.ok) continue;
      const { unmount } = render(<LienzoVista lienzo={r.lienzo} filas={[]} />);
      unmount();
    }
  });
});

const resumen: ResumenHoy = {
  hoy: "2026-10-22",
  adentro_contratistas: 19,
  adentro_proveedores: 1,
  ingresos_hoy: 7,
  praind_vencidos: 13,
  praind_por_vencer: 2,
  ingresos_por_dia: Array.from({ length: 14 }, (_, i) => ({
    dia: `2026-10-${String(9 + i).padStart(2, "0")}`,
    cantidad: i,
  })),
};

describe("ResumenDeHoy", () => {
  it("muestra los cuatro números y las alertas con singular y plural correctos", () => {
    const { container } = render(<ResumenDeHoy resumen={resumen} />);
    expect(screen.getByText("Contratistas adentro ahora")).toBeTruthy();
    expect([...container.querySelectorAll(".lienzo-kpi-valor")].map((n) => n.textContent)).toEqual([
      "19",
      "7",
      "13",
      "2",
    ]);
    expect(screen.getByText("1 proveedor")).toBeTruthy();
    expect(screen.getByText("13 contratistas")).toBeTruthy();
    expect(screen.getByText("2 PRAIND")).toBeTruthy();
  });

  it("dibuja los 14 días y marca el último como hoy", () => {
    const { container } = render(<ResumenDeHoy resumen={resumen} />);
    expect(container.querySelectorAll("rect.lienzo-barra")).toHaveLength(14);
    expect(container.querySelectorAll("rect.lienzo-barra-hoy")).toHaveLength(1);
    expect(screen.getByText("22·hoy")).toBeTruthy();
  });

  it("sin PRAIND vencidos ni por vencer no pide atención", () => {
    render(<ResumenDeHoy resumen={{ ...resumen, praind_vencidos: 0, praind_por_vencer: 0 }} />);
    expect(screen.getByText("pendiente por PRAIND")).toBeTruthy();
    expect(screen.queryByText(/con PRAIND vencido/)).toBeNull();
  });

  it("un solo vencido va en singular", () => {
    render(<ResumenDeHoy resumen={{ ...resumen, praind_vencidos: 1, praind_por_vencer: 1 }} />);
    expect(screen.getByText("1 contratista")).toBeTruthy();
    expect(screen.getByText("vence en 30 días")).toBeTruthy();
  });
});
