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

  it("tabla: barras sólo en columnas de número que estén en la tabla (o la cantidad de una agrupada)", () => {
    const tabla = (resto: Record<string, unknown>) =>
      validarLienzo({ fuente: "contratistas", filtros: [], widgets: [{ tipo: "tabla", columnas: ["nombre", "dias_para_vencer"], ...resto }] }).ok;
    expect(tabla({ barras: ["dias_para_vencer"] })).toBe(true);
    expect(tabla({ barras: ["nombre"] })).toBe(false);
    expect(tabla({ barras: ["acceso_activo"] })).toBe(false);
    expect(tabla({ barras: ["cantidad"] })).toBe(false);
    expect(tabla({ barras: ["cantidad"], agrupar_por: "empresa" })).toBe(true);
    expect(tabla({ barras: [] })).toBe(false);
  });

  it("tabla: la tendencia por fila va en una agrupada y con un campo de fecha", () => {
    const tabla = (resto: Record<string, unknown>) =>
      validarLienzo({ fuente: "ingresos", filtros: [], widgets: [{ tipo: "tabla", columnas: ["empresa"], ...resto }] }).ok;
    expect(tabla({ agrupar_por: "empresa", tendencia: "dia" })).toBe(true);
    expect(tabla({ tendencia: "dia" })).toBe(false);
    expect(tabla({ agrupar_por: "empresa", tendencia: "sitio" })).toBe(false);
  });

  it("'sumar' necesita un campo de número", () => {
    const consulta = (medida: string, campo?: string) =>
      validarLienzo({
        fuente: "consulta",
        consulta: { sql: "select 1", columnas: [{ nombre: "dia", tipo: "fecha" }, { nombre: "valor", tipo: "numero" }] },
        filtros: [],
        widgets: [{ tipo: "linea", eje_x: "dia", medida, ...(campo ? { campo } : {}) }],
      }).ok;
    expect(consulta("sumar", "valor")).toBe(true);
    expect(consulta("sumar")).toBe(false);
    expect(consulta("sumar", "dia")).toBe(false);
  });

  it("análisis en Python: su panel se revisa contra las columnas que declara", () => {
    const analisis = (resto: Record<string, unknown>) =>
      validarLienzo({
        fuente: "ingresos",
        filtros: [],
        widgets: [
          {
            tipo: "analisis",
            titulo: "Pronóstico",
            codigo: "resultado = []",
            columnas: [
              { nombre: "dia", tipo: "fecha" },
              { nombre: "esperado", tipo: "numero" },
            ],
            mostrar: { tipo: "linea", eje_x: "dia", medida: "sumar", campo: "esperado" },
            ...resto,
          },
        ],
      });
    expect(analisis({}).ok).toBe(true);
    // Un campo que el análisis no devuelve (aunque exista en la fuente del lienzo).
    expect(analisis({ mostrar: { tipo: "barras", eje_x: "empresa", medida: "contar" } }).ok).toBe(false);
    // Columnas repetidas, un panel que no es común, sin código.
    expect(analisis({ columnas: [{ nombre: "dia", tipo: "fecha" }, { nombre: "dia", tipo: "fecha" }] }).ok).toBe(false);
    expect(analisis({ mostrar: { tipo: "libre", titulo: "x", codigo: "return ''" } }).ok).toBe(false);
    expect(analisis({ codigo: "" }).ok).toBe(false);
    const errores = analisis({ mostrar: { tipo: "linea", eje_x: "nada", medida: "contar" } });
    expect(!errores.ok && errores.errores.join(" ")).toMatch(/widgets\.0\.mostrar\.eje_x/);
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

  describe('fuente "consulta"', () => {
    const consulta = {
      sql: "select empresa_nombre, count(*) as n from ia.ingresos group by 1",
      columnas: [
        { nombre: "empresa_nombre", tipo: "texto", etiqueta: "Empresa" },
        { nombre: "n", tipo: "numero" },
      ],
    };
    const doc = (extra: Doc = {}): Doc => ({
      fuente: "consulta",
      consulta,
      filtros: [],
      widgets: [{ tipo: "barras", eje_x: "empresa_nombre", medida: "contar" }],
      ...extra,
    });

    it("acepta un lienzo cuyos campos son las columnas de su consulta", () => {
      expect(validarLienzo(doc()).ok).toBe(true);
      const conFiltro = doc({ filtros: [{ campo: "n", op: "entre", valor: [1, 10] }] });
      expect(validarLienzo(conFiltro).ok).toBe(true);
    });

    it("rechaza campos que la consulta no devuelve", () => {
      const r = validarLienzo(doc({ widgets: [{ tipo: "barras", eje_x: "empresa", medida: "contar" }] }));
      expect(errores(r)).toContain("fuera del catálogo");
    });

    it("la consulta es obligatoria y sólo va con esta fuente", () => {
      expect(errores(validarLienzo(doc({ consulta: undefined })))).toContain("necesita su consulta");
      expect(errores(conCambio((d) => (d.consulta = consulta)))).toContain("Sólo la fuente");
    });

    it("rechaza columnas repetidas, nombres raros y tipos inventados", () => {
      const repetidas = { ...consulta, columnas: [consulta.columnas[0], consulta.columnas[0]] };
      expect(errores(validarLienzo(doc({ consulta: repetidas })))).toContain("repetidas");
      const rara = { ...consulta, columnas: [{ nombre: "Empresa; drop", tipo: "texto" }] };
      expect(validarLienzo(doc({ consulta: rara })).ok).toBe(false);
      const tipo = { ...consulta, columnas: [{ nombre: "x", tipo: "json" }] };
      expect(validarLienzo(doc({ consulta: tipo })).ok).toBe(false);
    });

    it("comparar y tendencia siguen siendo sólo de la fuente de ingresos", () => {
      const kpi = { tipo: "kpi", titulo: "t", medida: "contar", comparar: true };
      expect(validarLienzo(doc({ widgets: [kpi] })).ok).toBe(false);
    });
  });
});
