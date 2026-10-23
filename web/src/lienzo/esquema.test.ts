import { describe, expect, it } from "vitest";
import { validarLienzo } from "./esquema";

const valido = {
  fuente: "contratistas",
  filtros: [
    { campo: "estado_praind", op: "en", valor: ["VENCIDA", "POR_VENCER"] },
    { campo: "empresa", op: "no_en", valor: ["A", "B"] },
  ],
  widgets: [
    { tipo: "kpi", medida: "contar", titulo: "Por vencer" },
    { tipo: "barras", eje_x: "empresa", medida: "contar" },
    {
      tipo: "tabla",
      columnas: ["nombre", "empresa", "dias_para_vencer"],
      agrupar_por: "empresa",
      orden: { campo: "dias_para_vencer", dir: "asc" },
    },
  ],
};

// Documento mutable y sin tipar a propósito: los tests lo corrompen para probar el validador.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Doc = Record<string, any>;

function conCambio(cambio: (copia: Doc) => void) {
  const copia = structuredClone(valido) as Doc;
  cambio(copia);
  return validarLienzo(copia);
}

function errores(r: ReturnType<typeof validarLienzo>): string {
  return r.ok ? "" : r.errores.join("\n");
}

describe("validarLienzo", () => {
  it("acepta el documento de ejemplo del plan", () => {
    expect(validarLienzo(valido).ok).toBe(true);
  });

  it("rechaza lo que no es un objeto", () => {
    for (const x of [null, undefined, "x", 3, []]) expect(validarLienzo(x).ok).toBe(false);
  });

  it("rechaza propiedades desconocidas en cualquier nivel", () => {
    expect(conCambio((d) => (d.sql = "select 1")).ok).toBe(false);
    expect(conCambio((d) => (d.filtros[0].extra = 1)).ok).toBe(false);
    expect(conCambio((d) => (d.widgets[0].html = "<b>")).ok).toBe(false);
  });

  it("rechaza fuente, widget y operador fuera del catálogo", () => {
    expect(conCambio((d) => (d.fuente = "usuarios")).ok).toBe(false);
    expect(conCambio((d) => (d.widgets[0].tipo = "torta")).ok).toBe(false);
    expect(conCambio((d) => (d.filtros[0].op = "regex")).ok).toBe(false);
  });

  it("rechaza campos que no existen en la fuente", () => {
    expect(errores(conCambio((d) => (d.filtros[0].campo = "cedula")))).toContain("fuera del catálogo");
    expect(errores(conCambio((d) => (d.widgets[1].eje_x = "dia")))).toContain("fuera del catálogo");
    expect(errores(conCambio((d) => (d.widgets[2].columnas = ["constructor"])))).toContain("fuera del catálogo");
  });

  it("valida que el operador y el valor casen con el tipo del campo", () => {
    expect(conCambio((d) => (d.filtros[0] = { campo: "estado_praind", op: "contiene", valor: "VEN" })).ok).toBe(false);
    expect(conCambio((d) => (d.filtros[0] = { campo: "estado_praind", op: "igual", valor: "INVENTADO" })).ok).toBe(false);
    expect(conCambio((d) => (d.filtros[0] = { campo: "empresa", op: "en", valor: "A" })).ok).toBe(false);
    expect(conCambio((d) => (d.filtros[0] = { campo: "empresa", op: "en", valor: [] })).ok).toBe(false);
    expect(conCambio((d) => (d.filtros[0] = { campo: "empresa", op: "contiene", valor: "ACME" })).ok).toBe(true);
    expect(conCambio((d) => (d.filtros[0] = { campo: "acceso_activo", op: "igual", valor: true })).ok).toBe(true);
    expect(conCambio((d) => (d.filtros[0] = { campo: "acceso_activo", op: "igual", valor: "si" })).ok).toBe(false);
  });

  it("sólo admite los operadores que la función SQL del campo acepta", () => {
    const sitio = (op: string) => conCambio((d) => (d.filtros[0] = { campo: "sitio", op, valor: "Brisas" })).ok;
    expect(sitio("igual")).toBe(true);
    expect(sitio("contiene")).toBe(false);
    const nombre = (op: string) => conCambio((d) => (d.filtros[0] = { campo: "nombre", op, valor: "ana" })).ok;
    expect(nombre("contiene")).toBe(true);
    expect(nombre("igual")).toBe(false);
  });

  it("exige dos fechas válidas en 'entre'", () => {
    const entre = (valor: unknown) =>
      conCambio((d) => (d.filtros[0] = { campo: "vencimiento", op: "entre", valor })).ok;
    expect(entre(["2026-10-01", "2026-10-31"])).toBe(true);
    expect(entre(["2026-10-01"])).toBe(false);
    expect(entre(["2026-13-45", "2026-10-31"])).toBe(false);
    expect(entre(["01/10/2026", "31/10/2026"])).toBe(false);
  });

  it("exige su campo a las medidas que lo necesitan", () => {
    const kpi = (w: Record<string, unknown>) => conCambio((d) => (d.widgets[0] = { tipo: "kpi", titulo: "t", ...w })).ok;
    expect(kpi({ medida: "contar" })).toBe(true);
    expect(kpi({ medida: "contar", campo: "empresa" })).toBe(false);
    expect(kpi({ medida: "contar_distintos" })).toBe(false);
    expect(kpi({ medida: "contar_distintos", campo: "empresa" })).toBe(true);
    expect(kpi({ medida: "maximo", campo: "vencimiento" })).toBe(true);
    expect(kpi({ medida: "maximo", campo: "empresa" })).toBe(false);
  });

  it("permite ordenar una tabla agrupada por 'cantidad'", () => {
    const orden = (campo: string, agrupar: boolean) =>
      conCambio((d) => {
        if (!agrupar) delete d.widgets[2].agrupar_por;
        d.widgets[2].orden = { campo, dir: "desc" };
      }).ok;
    expect(orden("cantidad", true)).toBe(true);
    expect(orden("cantidad", false)).toBe(false);
  });

  it("limita el tamaño: sin widgets o con demasiados filas no pasa", () => {
    expect(conCambio((d) => (d.widgets = [])).ok).toBe(false);
    expect(conCambio((d) => (d.widgets[2].limite = 501)).ok).toBe(false);
    expect(conCambio((d) => (d.widgets[2].limite = 0)).ok).toBe(false);
  });

  it("no cae con claves heredadas del prototipo como campo", () => {
    expect(conCambio((d) => (d.filtros[0] = { campo: "__proto__", op: "igual", valor: "x" })).ok).toBe(false);
  });

  it("acepta la fuente de ingresos con sus campos", () => {
    const r = validarLienzo({
      fuente: "ingresos",
      filtros: [{ campo: "dia", op: "entre", valor: ["2026-10-09", "2026-10-22"] }],
      widgets: [{ tipo: "barras", eje_x: "dia", medida: "contar" }],
    });
    expect(r.ok).toBe(true);
  });
});
