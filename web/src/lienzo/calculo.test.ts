import { describe, expect, it } from "vitest";
import {
  textoDiasParaVencer,
  textoRangoDiasParaVencer,
  OTROS,
  calcularKpi,
  calcularPorciones,
  calcularSeries,
  etiquetaFecha,
  formatearFecha,
  grupoFecha,
  textoValor,
  type Fila,
} from "./calculo";
import type { WidgetBarras, WidgetLinea, WidgetTorta } from "./esquema";

const filas: Fila[] = [
  { nombre: "Ana", empresa: "A", vencimiento: "2026-10-05", dias_para_vencer: 3, acceso_activo: true },
  { nombre: "Beto", empresa: "A", vencimiento: "2026-10-20", dias_para_vencer: 18, acceso_activo: true },
  { nombre: "Cris", empresa: "B", vencimiento: null, dias_para_vencer: null, acceso_activo: false },
  { nombre: "Ana", empresa: null, vencimiento: "2026-11-01", dias_para_vencer: 30, acceso_activo: true },
];

describe("medida sumar", () => {
  it("suma sólo los números del campo (vacíos y textos no suman), sin ruido de decimales", () => {
    const valores: Fila[] = [{ v: 0.1 }, { v: 0.2 }, { v: null }, { v: "3" }, { v: 2 }];
    expect(calcularKpi({ tipo: "kpi", titulo: "t", medida: "sumar", campo: "v" }, valores)).toBe(2.3);
    expect(calcularKpi({ tipo: "kpi", titulo: "t", medida: "sumar", campo: "v" }, [])).toBe(0);
  });
});

describe("calcularKpi", () => {
  const kpi = (medida: "contar" | "contar_distintos" | "minimo" | "maximo", campo?: string) =>
    calcularKpi({ tipo: "kpi", titulo: "t", medida, campo }, filas);

  it("cuenta filas y valores distintos (sin contar los vacíos)", () => {
    expect(kpi("contar")).toBe(4);
    expect(kpi("contar_distintos", "nombre")).toBe(3);
    expect(kpi("contar_distintos", "empresa")).toBe(2);
  });

  it("mínimo y máximo de fechas y números, ignorando vacíos", () => {
    expect(kpi("minimo", "vencimiento")).toBe("2026-10-05");
    expect(kpi("maximo", "vencimiento")).toBe("2026-11-01");
    expect(kpi("minimo", "dias_para_vencer")).toBe(3);
    expect(kpi("maximo", "dias_para_vencer")).toBe(30);
  });

  it("sin filas: cuenta 0 y mínimo/máximo vacíos", () => {
    expect(calcularKpi({ tipo: "kpi", titulo: "t", medida: "contar" }, [])).toBe(0);
    expect(calcularKpi({ tipo: "kpi", titulo: "t", medida: "maximo", campo: "vencimiento" }, [])).toBeNull();
  });
});

const barras = (w: Partial<WidgetBarras>): WidgetBarras => ({ tipo: "barras", eje_x: "empresa", medida: "contar", ...w });
const linea = (w: Partial<WidgetLinea>): WidgetLinea => ({ tipo: "linea", eje_x: "dia", medida: "contar", ...w });

describe("calcularSeries", () => {
  it("una serie: agrupa por el eje, de mayor a menor, con '(sin dato)' para los vacíos al final", () => {
    const d = calcularSeries("contratistas", barras({}), filas, "Contratistas");
    expect(d.agrupacion).toBeNull();
    expect(d.categorias).toEqual(["A", "B", "(sin dato)"]);
    expect(d.series).toEqual([{ nombre: "Contratistas", valores: [2, 1, 1] }]);
  });

  it("un eje con orden propio va completo y en ese orden: las 24 horas, los siete días", () => {
    const ingresos: Fila[] = [
      { hora: 14, dia_semana: "Viernes" },
      { hora: 7, dia_semana: "Lunes" },
      { hora: 7, dia_semana: "Lunes" },
      { hora: 7, dia_semana: "Lunes" },
      { hora: 9, dia_semana: "Domingo" },
      { hora: 9, dia_semana: "Domingo" },
    ];
    const porHora = calcularSeries("ingresos", { tipo: "barras", eje_x: "hora", medida: "contar" }, ingresos);
    expect(porHora.categorias).toEqual(Array.from({ length: 24 }, (_, i) => String(i)));
    expect(porHora.series[0].valores.slice(6, 10)).toEqual([0, 3, 0, 2]);
    // Con límite 5 las horas igual van completas (no se juntan en "Otros").
    const conLimite = calcularSeries("ingresos", { tipo: "barras", eje_x: "hora", medida: "contar", limite: 5 }, ingresos);
    expect(conLimite.categorias).toHaveLength(24);
    const porDia = calcularSeries("ingresos", { tipo: "barras", eje_x: "dia_semana", medida: "contar" }, ingresos);
    expect(porDia.categorias).toEqual(["Lunes", "Martes", "Miércoles", "Jueves", "Viernes", "Sábado", "Domingo"]);
    expect(porDia.series[0].valores).toEqual([3, 0, 0, 0, 1, 0, 2]);
  });

  it("mide distintos por grupo", () => {
    const d = calcularSeries("contratistas", barras({ medida: "contar_distintos", campo: "nombre" }), filas);
    expect(d.series[0].valores[d.categorias.indexOf("A")]).toBe(2);
  });

  it("con un límite pedido es un top: las mayores, sin 'Otros'", () => {
    const muchas: Fila[] = Array.from({ length: 12 }, (_, i) => ({ empresa: `E${String(i).padStart(2, "0")}` }));
    muchas.push({ empresa: "E00" }, { empresa: "E00" });
    const d = calcularSeries("contratistas", barras({ limite: 3 }), muchas);
    expect(d.categorias).toEqual(["E00", "E01", "E02"]);
    expect(d.series[0].valores).toEqual([3, 1, 1]);
    expect(calcularSeries("contratistas", barras({ limite: 2 }), filas).categorias).toHaveLength(2);
  });

  it("sin límite, lo que pasa del máximo de categorías va a 'Otros', al final", () => {
    const muchas: Fila[] = Array.from({ length: 40 }, (_, i) => ({ empresa: `E${String(i).padStart(2, "0")}` }));
    muchas.push({ empresa: "E00" });
    const d = calcularSeries("contratistas", barras({}), muchas);
    expect(d.categorias).toHaveLength(31);
    expect(d.categorias[0]).toBe("E00");
    expect(d.categorias.at(-1)).toBe(OTROS);
    expect(d.series[0].valores.at(-1)).toBe(10);
  });

  it("no junta en 'Otros' una sola categoría sobrante: la muestra con su nombre", () => {
    const d = calcularSeries("contratistas", barras({}), [...Array.from({ length: 31 }, (_, i) => ({ empresa: `E${i}` }))]);
    expect(d.categorias).not.toContain(OTROS);
    expect(d.categorias).toHaveLength(31);
  });

  it("un eje de fecha va en orden y rellena con 0 los días sin datos", () => {
    const ingresos: Fila[] = [{ dia: "2026-10-12" }, { dia: "2026-10-09" }, { dia: "2026-10-12" }];
    const d = calcularSeries("ingresos", linea({}), ingresos);
    expect(d.agrupacion).toBe("dia");
    expect(d.categorias).toEqual(["2026-10-09", "2026-10-10", "2026-10-11", "2026-10-12"]);
    expect(d.series[0].valores).toEqual([1, 0, 0, 2]);
  });

  it("agrupa por semana (desde el lunes) y por mes", () => {
    const ingresos: Fila[] = [{ dia: "2026-10-19" }, { dia: "2026-10-25" }, { dia: "2026-10-26" }, { dia: "2026-11-02" }];
    const semana = calcularSeries("ingresos", linea({ agrupar_fecha: "semana" }), ingresos);
    // El 02/11/2026 es lunes: abre su propia semana.
    expect(semana.categorias).toEqual(["2026-10-19", "2026-10-26", "2026-11-02"]);
    expect(semana.series[0].valores).toEqual([2, 1, 1]);
    const mes = calcularSeries("ingresos", linea({ agrupar_fecha: "mes" }), ingresos);
    expect(mes.categorias).toEqual(["2026-10", "2026-11"]);
    expect(mes.series[0].valores).toEqual([3, 1]);
  });

  it("con `serie`, una serie por valor, la mayor primero, cada una con sus valores por categoría", () => {
    const ingresos: Fila[] = [
      { dia: "2026-10-01", sitio: "Brisas" },
      { dia: "2026-10-01", sitio: "Cartago" },
      { dia: "2026-10-02", sitio: "Brisas" },
    ];
    const d = calcularSeries("ingresos", linea({ serie: "sitio" }), ingresos);
    expect(d.series).toEqual([
      { nombre: "Brisas", valores: [1, 1] },
      { nombre: "Cartago", valores: [1, 0] },
    ]);
  });

  it("más series que colores: las menores van a 'Otros', marcada como tal", () => {
    const muchas: Fila[] = Array.from({ length: 10 }, (_, i) => ({ empresa: "A", sitio: `S${i}` }));
    const d = calcularSeries("ingresos", barras({ serie: "sitio" }), muchas);
    expect(d.series).toHaveLength(8);
    expect(d.series.at(-1)).toEqual({ nombre: OTROS, valores: [3], otros: true });
  });
});

describe("calcularPorciones", () => {
  const torta = (w: Partial<WidgetTorta>): WidgetTorta => ({ tipo: "torta", categoria: "empresa", medida: "contar", ...w });

  it("de mayor a menor, con 'Otros' al final y el total", () => {
    const datos: Fila[] = ["A", "A", "A", "B", "B", "C", "D"].map((empresa) => ({ empresa }));
    const { porciones, total } = calcularPorciones(torta({ limite: 2 }), datos);
    expect(porciones).toEqual([
      { nombre: "A", valor: 3 },
      { nombre: "B", valor: 2 },
      { nombre: OTROS, valor: 2, otros: true },
    ]);
    expect(total).toBe(7);
  });

  it("sin filas no hay porciones", () => {
    expect(calcularPorciones(torta({}), [])).toEqual({ porciones: [], total: 0 });
  });
});

describe("fechas", () => {
  it("grupoFecha: la semana empieza el lunes, también cruzando de mes", () => {
    expect(grupoFecha("2026-10-22", "semana")).toBe("2026-10-19");
    expect(grupoFecha("2026-11-01", "semana")).toBe("2026-10-26");
    expect(grupoFecha("2026-10-19", "semana")).toBe("2026-10-19");
    expect(grupoFecha("2026-10-22", "mes")).toBe("2026-10");
  });

  it("formatearFecha según el formato del lienzo", () => {
    expect(formatearFecha("2026-10-05")).toBe("05/10/2026");
    expect(formatearFecha("2026-10-05", "anio_mes_dia")).toBe("2026-10-05");
    expect(formatearFecha("2026-10-05", "texto")).toMatch(/^5 oct 2026$/);
    expect(formatearFecha("(sin dato)")).toBe("(sin dato)");
  });

  it("etiquetaFecha: corta para el eje", () => {
    expect(etiquetaFecha("2026-10-05", "dia")).toBe("05/10");
    expect(etiquetaFecha("2026-10-05", "dia", "anio_mes_dia")).toBe("10-05");
    expect(etiquetaFecha("2026-10-19", "semana")).toBe("sem. 19/10");
    expect(etiquetaFecha("2026-10", "mes")).toMatch(/oct.*2026/);
  });
});

describe("días para vencer en palabras", () => {
  it("un valor", () => {
    expect(textoDiasParaVencer(-40)).toBe("Vencido hace 40 días");
    expect(textoDiasParaVencer(0)).toBe("Vence hoy");
    expect(textoDiasParaVencer(17)).toBe("Faltan 17 días");
    expect(textoDiasParaVencer(Number.NaN)).toBe("—");
  });

  it("un rango (filtro entre), sin signos menos", () => {
    expect(textoRangoDiasParaVencer(-30, -1)).toBe("vencidos hace entre 1 y 30 días");
    expect(textoRangoDiasParaVencer(-30, 0)).toBe("vencidos en los últimos 30 días o que vencen hoy");
    expect(textoRangoDiasParaVencer(0, 30)).toBe("vencen en los próximos 30 días");
    expect(textoRangoDiasParaVencer(31, 60)).toBe("vencen dentro de 31 a 60 días");
    expect(textoRangoDiasParaVencer(-10, 10)).toBe("desde vencidos hace 10 días hasta los que vencen en 10 días");
    for (const [a, b] of [[-30, -1], [-30, 0], [0, 30], [-10, 10], [5, 5], [-5, -5]]) {
      expect(textoRangoDiasParaVencer(a, b)).not.toMatch(/-\d/);
    }
  });

});

describe("textoValor", () => {
  it("formatea vacíos y booleanos", () => {
    expect(textoValor(null)).toBe("—");
    expect(textoValor(undefined)).toBe("—");
    expect(textoValor(true)).toBe("Sí");
    expect(textoValor(false)).toBe("No");
    expect(textoValor(7)).toBe("7");
  });
});
