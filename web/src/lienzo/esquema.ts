import { z } from "../lib/validacion";
import {
  FUENTES,
  MEDIDAS,
  OPERADORES,
  OPERADORES_POR_TIPO,
  WIDGETS,
  definicionCampo,
  type DefinicionCampo,
  type Fuente,
} from "./catalogo";

/**
 * Documento JSON del lienzo (plan del asistente, Parte A, etapa 1). Estricto:
 * campos desconocidos, operadores que no van con el tipo del campo, valores
 * fuera del catálogo y medidas sin su campo se rechazan. Es la misma validación
 * que se aplicará a lo que devuelva el modelo en la etapa 2.
 */
const escalar = z.union([z.string().max(200), z.number().finite(), z.boolean()]);

const filtroEsquema = z
  .object({
    campo: z.string().min(1),
    op: z.enum(OPERADORES),
    valor: z.union([escalar, z.array(escalar).max(100)]),
  })
  .strict();

const ordenEsquema = z.object({ campo: z.string().min(1), dir: z.enum(["asc", "desc"]) }).strict();

const kpiEsquema = z
  .object({
    tipo: z.literal("kpi"),
    titulo: z.string().min(1).max(80),
    medida: z.enum(MEDIDAS),
    campo: z.string().min(1).optional(),
  })
  .strict();

const barrasEsquema = z
  .object({
    tipo: z.literal("barras"),
    titulo: z.string().min(1).max(80).optional(),
    eje_x: z.string().min(1),
    medida: z.enum(["contar", "contar_distintos"]),
    campo: z.string().min(1).optional(),
  })
  .strict();

const tablaEsquema = z
  .object({
    tipo: z.literal("tabla"),
    titulo: z.string().min(1).max(80).optional(),
    columnas: z.array(z.string().min(1)).min(1).max(12),
    agrupar_por: z.string().min(1).optional(),
    orden: ordenEsquema.optional(),
    limite: z.number().int().min(1).max(500).optional(),
  })
  .strict();

const widgetEsquema = z.discriminatedUnion("tipo", [kpiEsquema, barrasEsquema, tablaEsquema]);

const lienzoBase = z
  .object({
    fuente: z.enum(FUENTES),
    filtros: z.array(filtroEsquema).max(20),
    widgets: z.array(widgetEsquema).min(1).max(12),
  })
  .strict();

export type Filtro = z.infer<typeof filtroEsquema>;
export type Widget = z.infer<typeof widgetEsquema>;
export type WidgetKpi = z.infer<typeof kpiEsquema>;
export type WidgetBarras = z.infer<typeof barrasEsquema>;
export type WidgetTabla = z.infer<typeof tablaEsquema>;
export type Lienzo = z.infer<typeof lienzoBase>;

const FECHA = /^\d{4}-\d{2}-\d{2}$/;

function valorCoincideConTipo(def: DefinicionCampo, valor: unknown): boolean {
  switch (def.tipo) {
    case "texto":
      return typeof valor === "string";
    case "enumeracion":
      return typeof valor === "string" && (def.valores ?? []).includes(valor);
    case "fecha":
      return typeof valor === "string" && FECHA.test(valor) && !Number.isNaN(Date.parse(valor));
    case "numero":
      return typeof valor === "number";
    case "booleano":
      return typeof valor === "boolean";
  }
}

function revisarFiltro(fuente: Fuente, filtro: Filtro, ruta: (string | number)[], ctx: z.RefinementCtx) {
  const def = definicionCampo(fuente, filtro.campo);
  if (!def) {
    ctx.addIssue({ code: "custom", path: [...ruta, "campo"], message: `Campo "${filtro.campo}" fuera del catálogo` });
    return;
  }
  if (!(def.operadores ?? OPERADORES_POR_TIPO[def.tipo]).includes(filtro.op)) {
    ctx.addIssue({ code: "custom", path: [...ruta, "op"], message: `"${filtro.op}" no aplica a "${filtro.campo}"` });
    return;
  }
  const { op, valor } = filtro;
  const lista = Array.isArray(valor);
  const valores: unknown[] = lista ? valor : [valor];
  const forma =
    op === "entre" ? lista && valor.length === 2 : op === "en" || op === "no_en" ? lista && valor.length > 0 : !lista;
  if (!forma || !valores.every((v) => valorCoincideConTipo(def, v))) {
    ctx.addIssue({ code: "custom", path: [...ruta, "valor"], message: `Valor inválido para "${filtro.campo}" con "${op}"` });
  }
}

function exigirCampo(fuente: Fuente, campo: string, ruta: (string | number)[], ctx: z.RefinementCtx) {
  if (!definicionCampo(fuente, campo)) {
    ctx.addIssue({ code: "custom", path: ruta, message: `Campo "${campo}" fuera del catálogo` });
  }
}

function revisarWidget(fuente: Fuente, w: Widget, i: number, ctx: z.RefinementCtx) {
  const ruta = ["widgets", i];
  if (w.tipo === "kpi" || w.tipo === "barras") {
    if (w.campo) exigirCampo(fuente, w.campo, [...ruta, "campo"], ctx);
    if (w.medida === "contar" && w.campo) {
      ctx.addIssue({ code: "custom", path: [...ruta, "campo"], message: '"contar" no lleva campo' });
    }
    if (w.medida !== "contar" && !w.campo) {
      ctx.addIssue({ code: "custom", path: [...ruta, "campo"], message: `"${w.medida}" necesita un campo` });
    }
    if (w.campo && (w.medida === "minimo" || w.medida === "maximo")) {
      const tipo = definicionCampo(fuente, w.campo)?.tipo;
      if (tipo && tipo !== "fecha" && tipo !== "numero") {
        ctx.addIssue({ code: "custom", path: [...ruta, "campo"], message: `"${w.medida}" sólo sirve con fecha o número` });
      }
    }
  }
  if (w.tipo === "barras") exigirCampo(fuente, w.eje_x, [...ruta, "eje_x"], ctx);
  if (w.tipo === "tabla") {
    w.columnas.forEach((c, j) => exigirCampo(fuente, c, [...ruta, "columnas", j], ctx));
    if (w.agrupar_por) exigirCampo(fuente, w.agrupar_por, [...ruta, "agrupar_por"], ctx);
    // Agrupada, la tabla sólo ordena por el campo de grupo o por la cantidad.
    if (w.orden && !(w.agrupar_por && w.orden.campo === "cantidad")) {
      exigirCampo(fuente, w.orden.campo, [...ruta, "orden", "campo"], ctx);
    }
  }
}

export const lienzoEsquema = lienzoBase.superRefine((lienzo, ctx) => {
  lienzo.filtros.forEach((f, i) => revisarFiltro(lienzo.fuente, f, ["filtros", i], ctx));
  lienzo.widgets.forEach((w, i) => revisarWidget(lienzo.fuente, w, i, ctx));
});

export type ResultadoValidacion = { ok: true; lienzo: Lienzo } | { ok: false; errores: string[] };

export function validarLienzo(entrada: unknown): ResultadoValidacion {
  const resultado = lienzoEsquema.safeParse(entrada);
  if (resultado.success) return { ok: true, lienzo: resultado.data };
  return {
    ok: false,
    errores: resultado.error.issues.map((e) => `${e.path.join(".") || "(raíz)"}: ${e.message}`),
  };
}

/** Los tipos de widget que el renderizador sabe dibujar. */
export const TIPOS_WIDGET = WIDGETS;
