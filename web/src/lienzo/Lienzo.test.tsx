import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { ResumenHoy } from "../api/lienzo";
import { hoyCostaRica, sumarDias, type Fila } from "./calculo";
import LienzoVista from "./Lienzo";
import ResumenDeHoy from "./ResumenHoy";
import { validarLienzo } from "./esquema";
import { VISTAS_PREDEFINIDAS } from "./vistas";

const contratistas: Fila[] = [
  { nombre: "Ana", empresa: "ACME", vencimiento: "2026-10-05", dias_para_vencer: -3, tipo_ingreso: "PRAIND" },
  { nombre: "Beto", empresa: "ACME", vencimiento: "2026-10-01", dias_para_vencer: -7, tipo_ingreso: "PRAIND" },
  { nombre: "Cris", empresa: "OTRA", vencimiento: "2026-09-01", dias_para_vencer: -40, tipo_ingreso: "SWAT" },
];

const ingresos: Fila[] = [
  { dia: "2026-10-20", nombre: "Ana", empresa: "ACME", tipo_ingreso: "PRAIND", sitio: "Brisas" },
  { dia: "2026-10-21", nombre: "Beto", empresa: "ACME", tipo_ingreso: "PRAIND", sitio: "Cartago" },
  { dia: "2026-10-22", nombre: "Cris", empresa: "OTRA", tipo_ingreso: "SWAT", sitio: "Brisas" },
];

function lienzo(widgets: unknown[], extra: Record<string, unknown> = {}, fuente = "contratistas") {
  const r = validarLienzo({ fuente, filtros: [], widgets, ...extra });
  if (!r.ok) throw new Error(r.errores.join("; "));
  return r.lienzo;
}

describe("LienzoVista", () => {
  it("dibuja un KPI con su título y valor, y una fecha en el formato pedido", () => {
    render(
      <LienzoVista
        lienzo={lienzo(
          [
            { tipo: "kpi", titulo: "Vencidos", medida: "contar" },
            { tipo: "kpi", titulo: "Último", medida: "maximo", campo: "vencimiento" },
          ],
          { formato: { fecha: "anio_mes_dia" } },
        )}
        filas={contratistas}
      />,
    );
    expect(screen.getByText("Vencidos")).toBeTruthy();
    expect(screen.getByText("3")).toBeTruthy();
    expect(screen.getByText("2026-10-05")).toBeTruthy();
  });

  it("barras horizontales: una por categoría, con su valor en la punta y nombre accesible", () => {
    const { container } = render(
      <LienzoVista lienzo={lienzo([{ tipo: "barras", titulo: "Por empresa", eje_x: "empresa", medida: "contar" }])} filas={contratistas} />,
    );
    expect(screen.getByRole("img", { name: /Por empresa/ })).toBeTruthy();
    expect(container.querySelectorAll("g.lienzo-categoria")).toHaveLength(2);
    expect(container.querySelectorAll(".lienzo-valor-texto")).toHaveLength(2);
  });

  it("el tooltip responde al teclado: foco y flechas recorren las categorías", () => {
    render(<LienzoVista lienzo={lienzo([{ tipo: "barras", titulo: "Por empresa", eje_x: "empresa", medida: "contar" }])} filas={contratistas} />);
    const grafico = screen.getByRole("img", { name: /Por empresa/ });
    fireEvent.focus(grafico);
    expect(screen.getByRole("status").textContent).toContain("ACME");
    fireEvent.keyDown(grafico, { key: "ArrowRight" });
    expect(screen.getByRole("status").textContent).toContain("OTRA");
    fireEvent.keyDown(grafico, { key: "Escape" });
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("cada gráfico se puede ver como tabla, y volver", () => {
    render(<LienzoVista lienzo={lienzo([{ tipo: "barras", titulo: "Por empresa", eje_x: "empresa", medida: "contar" }])} filas={contratistas} />);
    fireEvent.click(screen.getByRole("button", { name: "Ver Por empresa como tabla" }));
    const tabla = screen.getByRole("table");
    expect(within(tabla).getAllByRole("row")).toHaveLength(3);
    expect(within(tabla).getByRole("cell", { name: "ACME" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Ver Por empresa como gráfico" }));
    expect(screen.queryByRole("table")).toBeNull();
  });

  it("barras apiladas: una leyenda con cada serie", () => {
    render(
      <LienzoVista
        lienzo={lienzo([{ tipo: "barras", eje_x: "empresa", serie: "tipo_ingreso", medida: "contar" }])}
        filas={contratistas}
      />,
    );
    const leyenda = screen.getByRole("list");
    expect(within(leyenda).getAllByRole("listitem").map((li) => li.textContent)).toEqual(["PRAIND", "SWAT"]);
  });

  it("línea: un trazo por serie y leyenda cuando son varias", () => {
    const { container } = render(
      <LienzoVista lienzo={lienzo([{ tipo: "linea", eje_x: "dia", serie: "sitio", medida: "contar" }], {}, "ingresos")} filas={ingresos} />,
    );
    expect(container.querySelectorAll("path.lienzo-linea")).toHaveLength(2);
    expect(within(screen.getByRole("list")).getAllByRole("listitem")).toHaveLength(2);
  });

  it("torta: una porción por categoría, leyenda con valor y porcentaje, total al centro", () => {
    const { container } = render(
      <LienzoVista lienzo={lienzo([{ tipo: "torta", categoria: "tipo_ingreso", medida: "contar" }])} filas={contratistas} />,
    );
    expect(container.querySelectorAll("path.lienzo-porcion")).toHaveLength(2);
    const items = within(screen.getByRole("list")).getAllByRole("listitem").map((li) => li.textContent);
    expect(items[0]).toMatch(/PRAIND.*2.*66[,.]7/);
    expect(container.querySelector(".lienzo-torta-total")?.textContent).toBe("3");
  });

  it("enfoque: los paneles van a lo ancho, uno debajo del otro", () => {
    const { container } = render(
      <LienzoVista
        lienzo={lienzo([{ tipo: "torta", categoria: "tipo_ingreso", medida: "contar", ancho: "tercio" }], { disposicion: "enfoque" })}
        filas={contratistas}
      />,
    );
    expect(container.querySelector(".lienzo-enfoque")).toBeTruthy();
    expect(container.querySelector(".lienzo-panel")?.className).toContain("lienzo-ancho-completo");
  });

  it("cuadrícula: respeta el ancho pedido o el de cada tipo", () => {
    const { container } = render(
      <LienzoVista
        lienzo={lienzo([
          { tipo: "kpi", titulo: "t", medida: "contar" },
          { tipo: "torta", categoria: "tipo_ingreso", medida: "contar", ancho: "mitad" },
        ])}
        filas={contratistas}
      />,
    );
    const paneles = [...container.querySelectorAll(".lienzo-panel")].map((p) => p.className);
    expect(paneles[0]).toContain("lienzo-ancho-cuarto");
    expect(paneles[1]).toContain("lienzo-ancho-mitad");
  });

  it("sin datos muestra un aviso en vez de un gráfico vacío", () => {
    render(<LienzoVista lienzo={lienzo([{ tipo: "torta", categoria: "empresa", medida: "contar" }])} filas={[]} />);
    expect(screen.getByText("Sin datos para mostrar.")).toBeTruthy();
  });

  it("el texto de los datos se muestra como texto, nunca como HTML", () => {
    const peligrosa: Fila[] = [{ nombre: "<img src=x onerror=alert(1)>", empresa: "<b>X</b>" }];
    const { container } = render(
      <LienzoVista
        lienzo={lienzo([
          { tipo: "tabla", columnas: ["nombre", "empresa"] },
          { tipo: "torta", categoria: "empresa", medida: "contar" },
        ])}
        filas={peligrosa}
      />,
    );
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("b")).toBeNull();
    expect(screen.getByText("<img src=x onerror=alert(1)>")).toBeTruthy();
  });

  it("todas las vistas de ejemplo se dibujan con datos", () => {
    for (const vista of VISTAS_PREDEFINIDAS) {
      const r = validarLienzo(vista.lienzo);
      if (!r.ok) throw new Error(`${vista.id}: ${r.errores.join("; ")}`);
      const filas = r.lienzo.fuente === "ingresos" ? ingresos : contratistas;
      const { unmount, container } = render(<LienzoVista lienzo={r.lienzo} filas={filas} />);
      expect(container.querySelectorAll(".lienzo-panel")).toHaveLength(r.lienzo.widgets.length);
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
    cantidad: i + 1,
  })),
};

describe("ResumenDeHoy", () => {
  it("muestra los cuatro números y las alertas con singular y plural correctos", () => {
    const { container } = render(<ResumenDeHoy resumen={resumen} />);
    expect(screen.getByText("Contratistas adentro ahora")).toBeTruthy();
    expect([...container.querySelectorAll(".lienzo-kpi-valor")].map((n) => n.textContent)).toEqual(["19", "7", "13", "2"]);
    expect(screen.getByText("1 proveedor")).toBeTruthy();
    expect(screen.getByText("13 contratistas")).toBeTruthy();
    expect(screen.getByText("2 PRAIND")).toBeTruthy();
  });

  it("dibuja los 14 días y marca el último como hoy", () => {
    const { container } = render(<ResumenDeHoy resumen={resumen} />);
    expect(container.querySelectorAll("g.lienzo-categoria")).toHaveLength(14);
    expect(container.querySelectorAll("path.lienzo-barra-hoy")).toHaveLength(1);
    expect(screen.getByText("22/10 · hoy")).toBeTruthy();
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

  it("ingresos de hoy contra la semana pasada A ESTA HORA, en cantidades (nunca -100 %)", () => {
    // 22/10/2026 es jueves. Hoy van 7; el jueves pasado a esta hora iban 12.
    const { container } = render(<ResumenDeHoy resumen={{ ...resumen, ingresos_hoy: 7, ingresos_semana_pasada_a_esta_hora: 12 }} />);
    expect(screen.getByText("5 menos que el jueves pasado a esta hora (12)")).toBeTruthy();
    // Al arrancar el día (0 contra 0) no aparece ningún porcentaje ni signo menos.
    render(<ResumenDeHoy resumen={{ ...resumen, ingresos_hoy: 0, ingresos_semana_pasada_a_esta_hora: 0 }} />);
    expect(screen.getByText("Igual que el jueves pasado a esta hora (0)")).toBeTruthy();
    expect(container.textContent).not.toMatch(/-\d+\s?%/);
  });

  it("sin el dato de la semana pasada a esta hora (función vieja), no compara contra el día completo", () => {
    render(<ResumenDeHoy resumen={{ ...resumen, ingresos_hoy: 0 }} />);
    expect(screen.queryByText(/%/)).toBeNull();
    expect(screen.queryByText(/semana pasada/)).toBeNull();
  });
});

describe("KPI que compara contra el período anterior", () => {
  it("si el rango termina hoy, compara sólo días completos (hoy va a medias) y lo dice", () => {
    const hoy = hoyCostaRica();
    const ayer = sumarDias(hoy, -1);
    // Rango de 2 días (ayer y hoy): ayer 3 ingresos, hoy 1 (a medias). Antes: el día previo con 3.
    const filas = [{ dia: ayer }, { dia: ayer }, { dia: ayer }, { dia: hoy }];
    const anteriores = [{ dia: sumarDias(hoy, -3) }, { dia: sumarDias(hoy, -2) }, { dia: sumarDias(hoy, -2) }, { dia: sumarDias(hoy, -2) }];
    render(
      <LienzoVista
        lienzo={lienzo([{ tipo: "kpi", titulo: "Ingresos", medida: "contar", comparar: true }], {}, "ingresos")}
        filas={filas}
        anteriores={anteriores}
        rango={{ desde: ayer, hasta: hoy }}
      />,
    );
    // Ayer (3) contra el día justo anterior (antier: 3): sin cambio. No se mezcla el día de hace 3 (1)
    // ni el de hoy a medias (que con el período completo daría una caída que no existe).
    const texto = screen.getByText(/sin contar hoy/).textContent ?? "";
    expect(texto).toMatch(/^Igual que el día anterior, sin contar hoy/);
    expect(texto).toContain("(3)");
  });

  it("si el rango es sólo hoy, no compara (un día a medias contra uno completo no dice nada)", () => {
    const hoy = hoyCostaRica();
    render(
      <LienzoVista
        lienzo={lienzo([{ tipo: "kpi", titulo: "Hoy", medida: "contar", comparar: true }], {}, "ingresos")}
        filas={[{ dia: hoy }]}
        anteriores={[{ dia: sumarDias(hoy, -1) }, { dia: sumarDias(hoy, -1) }]}
        rango={{ desde: hoy, hasta: hoy }}
      />,
    );
    expect(screen.queryByText(/vs\./)).toBeNull();
  });

  it("un KPI de días para vencer se escribe en palabras", () => {
    render(<LienzoVista lienzo={lienzo([{ tipo: "kpi", titulo: "Más vencido", medida: "minimo", campo: "dias_para_vencer" }])} filas={contratistas} />);
    expect(screen.getByText("Vencido hace 40 días")).toBeTruthy();
  });
});
