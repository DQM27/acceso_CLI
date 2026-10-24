import { describe, expect, it } from "vitest";
import { validarLienzo } from "./esquema";
import { VISTAS_PREDEFINIDAS } from "./vistas";

/** Las herramientas que el modelo podrá usar: gráficos, disposición y formato. */
function con(widgets: unknown[], extra: Record<string, unknown> = {}, fuente = "ingresos") {
  return validarLienzo({ fuente, filtros: [], widgets, ...extra });
}
function errores(r: ReturnType<typeof validarLienzo>): string {
  return r.ok ? "" : r.errores.join("\n");
}

describe("herramientas del lienzo", () => {
  it("torta y dona: categoría del catálogo y porciones acotadas", () => {
    expect(con([{ tipo: "torta", categoria: "tipo_ingreso", medida: "contar" }]).ok).toBe(true);
    expect(con([{ tipo: "torta", categoria: "empresa", medida: "contar", dona: false, limite: 4 }]).ok).toBe(true);
    expect(con([{ tipo: "torta", categoria: "cedula", medida: "contar" }]).ok).toBe(false);
    expect(con([{ tipo: "torta", categoria: "empresa", medida: "contar", limite: 20 }]).ok).toBe(false);
    expect(con([{ tipo: "torta", categoria: "empresa", medida: "minimo", campo: "dia" }]).ok).toBe(false);
  });

  it("línea: sólo sobre un eje de fecha, con área sólo si es una sola serie", () => {
    expect(con([{ tipo: "linea", eje_x: "dia", medida: "contar", area: true }]).ok).toBe(true);
    expect(con([{ tipo: "linea", eje_x: "dia", medida: "contar", serie: "sitio" }]).ok).toBe(true);
    expect(errores(con([{ tipo: "linea", eje_x: "empresa", medida: "contar" }]))).toContain("eje de fecha");
    expect(errores(con([{ tipo: "linea", eje_x: "dia", medida: "contar", serie: "sitio", area: true }]))).toContain(
      "una sola línea",
    );
  });

  it("barras: orientación, apiladas por una serie y agrupadas por semana o mes", () => {
    expect(con([{ tipo: "barras", eje_x: "empresa", medida: "contar", orientacion: "horizontal", limite: 10 }]).ok).toBe(true);
    expect(con([{ tipo: "barras", eje_x: "empresa", serie: "tipo_ingreso", medida: "contar" }]).ok).toBe(true);
    expect(con([{ tipo: "barras", eje_x: "dia", agrupar_fecha: "semana", medida: "contar" }]).ok).toBe(true);
    expect(errores(con([{ tipo: "barras", eje_x: "empresa", agrupar_fecha: "mes", medida: "contar" }]))).toContain(
      "eje de fecha",
    );
    expect(errores(con([{ tipo: "barras", eje_x: "empresa", serie: "empresa", medida: "contar" }]))).toContain(
      "mismo campo",
    );
    expect(errores(con([{ tipo: "barras", eje_x: "empresa", serie: "dia", medida: "contar" }]))).toContain("fecha");
  });

  it("disposición: cuadrícula o enfoque, y ancho por panel", () => {
    const torta = { tipo: "torta", categoria: "tipo_ingreso", medida: "contar" };
    expect(con([torta], { disposicion: "enfoque" }).ok).toBe(true);
    expect(con([{ ...torta, ancho: "dos_tercios" }], { disposicion: "cuadricula" }).ok).toBe(true);
    expect(con([torta], { disposicion: "carrusel" }).ok).toBe(false);
    expect(con([{ ...torta, ancho: "gigante" }]).ok).toBe(false);
  });

  it("formato de fecha y título del lienzo", () => {
    const kpi = { tipo: "kpi", titulo: "t", medida: "contar" };
    expect(con([kpi], { titulo: "Mi vista", formato: { fecha: "texto" } }).ok).toBe(true);
    expect(con([kpi], { formato: { fecha: "mm/dd/aaaa" } }).ok).toBe(false);
    expect(con([kpi], { formato: { hora: "12h" } }).ok).toBe(false);
  });

  it("KPI con comparación y tendencia: sólo con datos por día y al contar", () => {
    expect(con([{ tipo: "kpi", titulo: "t", medida: "contar", comparar: true, tendencia: true }]).ok).toBe(true);
    expect(errores(con([{ tipo: "kpi", titulo: "t", medida: "contar", comparar: true }], {}, "contratistas"))).toContain(
      "datos por día",
    );
    expect(errores(con([{ tipo: "kpi", titulo: "t", medida: "maximo", campo: "dia", tendencia: true }]))).toContain("al contar");
  });

  it("KPI con tono de estado", () => {
    expect(con([{ tipo: "kpi", titulo: "t", medida: "contar", tono: "exito" }]).ok).toBe(true);
    expect(con([{ tipo: "kpi", titulo: "t", medida: "contar", tono: "fucsia" }]).ok).toBe(false);
  });

  it("todas las vistas de ejemplo pasan el validador", () => {
    for (const vista of VISTAS_PREDEFINIDAS) {
      const r = validarLienzo(vista.lienzo);
      expect(r.ok, `${vista.id}: ${errores(r)}`).toBe(true);
    }
  });
});
