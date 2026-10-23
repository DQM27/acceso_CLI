// Una sola fuente para el panel web (que lo importa desde aquí; Vite traduce
// `npm:zod@…` a su zod) y para la Edge Function `lienzo-ia`: los dos validan
// el lienzo con el mismo código.
import { z } from "npm:zod@4.6.5";
import {
  AGRUPACIONES_FECHA,
  ANCHOS,
  DISPOSICIONES,
  FORMATOS_FECHA,
  FUENTES,
  MAXIMO_CATEGORIAS,
  MAXIMO_SERIES,
  MEDIDAS,
  OPERADORES,
  OPERADORES_POR_TIPO,
  TONOS,
  definicionCampo,
  type DefinicionCampo,
  type Fuente,
} from "./catalogo.ts";

// Sin compilación dinámica: la CSP estricta del panel la bloquea (igual que en
// `web/src/lib/validacion.ts`).
z.config({ jitless: true });

/**
 * Documento JSON del lienzo (plan del asistente, Parte A). Estricto: campos
 * desconocidos, operadores que no van con el tipo del campo, valores fuera del
 * catálogo y medidas sin su campo se rechazan. Es la misma validación que se
 * aplicará a lo que devuelva el modelo en la etapa 2.
 */
const escalar = z.union([z.string().max(200), z.number().finite(), z.boolean()]);
const titulo = z.string().min(1).max(80);
const campo = z.string().min(1).max(40);

const filtroEsquema = z
  .object({
    campo,
    op: z.enum(OPERADORES),
    valor: z.union([escalar, z.array(escalar).max(100)]),
  })
  .strict();

const ordenEsquema = z.object({ campo, dir: z.enum(["asc", "desc"]) }).strict();

/** Lo que comparten todos los paneles. */
const panel = { ancho: z.enum(ANCHOS).optional() };
/** Medidas que sirven para comparar grupos (un gráfico no muestra mínimos). */
const medidaDeGrupo = z.enum(["contar", "contar_distintos"]);

const kpiEsquema = z
  .object({
    tipo: z.literal("kpi"),
    titulo,
    medida: z.enum(MEDIDAS),
    campo: campo.optional(),
    tono: z.enum(TONOS).optional(),
    /** Contra el período anterior del mismo largo ("+12 % vs. las 2 semanas anteriores"). */
    comparar: z.boolean().optional(),
    /** Mini línea con el valor día por día. */
    tendencia: z.boolean().optional(),
    ...panel,
  })
  .strict();

const barrasEsquema = z
  .object({
    tipo: z.literal("barras"),
    titulo: titulo.optional(),
    eje_x: campo,
    medida: medidaDeGrupo,
    campo: campo.optional(),
    /** Segunda dimensión: parte cada barra en segmentos apilados. */
    serie: campo.optional(),
    orientacion: z.enum(["vertical", "horizontal"]).optional(),
    agrupar_fecha: z.enum(AGRUPACIONES_FECHA).optional(),
    limite: z.number().int().min(1).max(MAXIMO_CATEGORIAS).optional(),
    ...panel,
  })
  .strict();

const lineaEsquema = z
  .object({
    tipo: z.literal("linea"),
    titulo: titulo.optional(),
    eje_x: campo,
    medida: medidaDeGrupo,
    campo: campo.optional(),
    /** Una línea por cada valor de este campo. */
    serie: campo.optional(),
    agrupar_fecha: z.enum(AGRUPACIONES_FECHA).optional(),
    /** Relleno suave bajo la línea (sólo con una serie). */
    area: z.boolean().optional(),
    ...panel,
  })
  .strict();

const tortaEsquema = z
  .object({
    tipo: z.literal("torta"),
    titulo: titulo.optional(),
    categoria: campo,
    medida: medidaDeGrupo,
    campo: campo.optional(),
    /** Porciones con nombre; el resto va a "Otros". */
    limite: z.number().int().min(2).max(MAXIMO_SERIES).optional(),
    /** Dona (con el total al centro) en vez de torta llena. */
    dona: z.boolean().optional(),
    ...panel,
  })
  .strict();

const tablaEsquema = z
  .object({
    tipo: z.literal("tabla"),
    titulo: titulo.optional(),
    columnas: z.array(campo).min(1).max(12),
    agrupar_por: campo.optional(),
    orden: ordenEsquema.optional(),
    limite: z.number().int().min(1).max(500).optional(),
    ...panel,
  })
  .strict();

const widgetEsquema = z.discriminatedUnion("tipo", [
  kpiEsquema,
  barrasEsquema,
  lineaEsquema,
  tortaEsquema,
  tablaEsquema,
]);

const lienzoBase = z
  .object({
    fuente: z.enum(FUENTES),
    titulo: titulo.optional(),
    disposicion: z.enum(DISPOSICIONES).optional(),
    /** Presentación de los valores; no cambia los datos. */
    formato: z.object({ fecha: z.enum(FORMATOS_FECHA).optional() }).strict().optional(),
    filtros: z.array(filtroEsquema).max(20),
    widgets: z.array(widgetEsquema).min(1).max(12),
  })
  .strict();

export type Filtro = z.infer<typeof filtroEsquema>;
export type Widget = z.infer<typeof widgetEsquema>;
export type WidgetKpi = z.infer<typeof kpiEsquema>;
export type WidgetBarras = z.infer<typeof barrasEsquema>;
export type WidgetLinea = z.infer<typeof lineaEsquema>;
export type WidgetTorta = z.infer<typeof tortaEsquema>;
export type WidgetTabla = z.infer<typeof tablaEsquema>;
export type Lienzo = z.infer<typeof lienzoBase>;

type Ruta = (string | number)[];

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

function problema(ctx: z.RefinementCtx, path: Ruta, message: string) {
  ctx.addIssue({ code: "custom", path, message });
}

function revisarFiltro(fuente: Fuente, filtro: Filtro, ruta: Ruta, ctx: z.RefinementCtx) {
  const def = definicionCampo(fuente, filtro.campo);
  if (!def) return problema(ctx, [...ruta, "campo"], `Campo "${filtro.campo}" fuera del catálogo`);
  if (!(def.operadores ?? OPERADORES_POR_TIPO[def.tipo]).includes(filtro.op)) {
    return problema(ctx, [...ruta, "op"], `"${filtro.op}" no aplica a "${filtro.campo}"`);
  }
  const { op, valor } = filtro;
  const lista = Array.isArray(valor);
  const valores: unknown[] = lista ? valor : [valor];
  const forma =
    op === "entre" ? lista && valor.length === 2 : op === "en" || op === "no_en" ? lista && valor.length > 0 : !lista;
  if (!forma || !valores.every((v) => valorCoincideConTipo(def, v))) {
    problema(ctx, [...ruta, "valor"], `Valor inválido para "${filtro.campo}" con "${op}"`);
  }
}

/** Devuelve la definición del campo, o anota el problema si no existe. */
function exigirCampo(fuente: Fuente, nombre: string, ruta: Ruta, ctx: z.RefinementCtx) {
  const def = definicionCampo(fuente, nombre);
  if (!def) problema(ctx, ruta, `Campo "${nombre}" fuera del catálogo`);
  return def;
}

function revisarMedida(
  fuente: Fuente,
  w: { medida: string; campo?: string },
  ruta: Ruta,
  ctx: z.RefinementCtx,
) {
  if (w.campo) exigirCampo(fuente, w.campo, [...ruta, "campo"], ctx);
  if (w.medida === "contar" && w.campo) problema(ctx, [...ruta, "campo"], '"contar" no lleva campo');
  if (w.medida !== "contar" && !w.campo) problema(ctx, [...ruta, "campo"], `"${w.medida}" necesita un campo`);
  if (w.campo && (w.medida === "minimo" || w.medida === "maximo")) {
    const tipo = definicionCampo(fuente, w.campo)?.tipo;
    if (tipo && tipo !== "fecha" && tipo !== "numero") {
      problema(ctx, [...ruta, "campo"], `"${w.medida}" sólo sirve con fecha o número`);
    }
  }
}

/** Eje y serie de un gráfico de barras o de línea. */
function revisarEjes(fuente: Fuente, w: WidgetBarras | WidgetLinea, ruta: Ruta, ctx: z.RefinementCtx) {
  const eje = exigirCampo(fuente, w.eje_x, [...ruta, "eje_x"], ctx);
  if (eje && w.tipo === "linea" && eje.tipo !== "fecha") {
    problema(ctx, [...ruta, "eje_x"], "Una línea necesita un eje de fecha");
  }
  if (eje && w.agrupar_fecha && eje.tipo !== "fecha") {
    problema(ctx, [...ruta, "agrupar_fecha"], "Sólo se agrupa por día, semana o mes un eje de fecha");
  }
  if (w.serie) {
    const serie = exigirCampo(fuente, w.serie, [...ruta, "serie"], ctx);
    if (w.serie === w.eje_x) problema(ctx, [...ruta, "serie"], "La serie no puede ser el mismo campo del eje");
    else if (serie?.tipo === "fecha") problema(ctx, [...ruta, "serie"], "La serie no puede ser una fecha");
  }
  if (w.tipo === "linea" && w.area && w.serie) {
    problema(ctx, [...ruta, "area"], "El área sólo se usa con una sola línea");
  }
}

function revisarWidget(fuente: Fuente, w: Widget, i: number, ctx: z.RefinementCtx) {
  const ruta = ["widgets", i];
  switch (w.tipo) {
    case "kpi":
      revisarMedida(fuente, w, ruta, ctx);
      // Comparar y la tendencia necesitan un día por fila: hoy sólo los ingresos lo tienen.
      if ((w.comparar || w.tendencia) && definicionCampo(fuente, "dia")?.tipo !== "fecha") {
        problema(ctx, ruta, "Comparar y la tendencia sólo sirven con datos por día (ingresos)");
      }
      if ((w.comparar || w.tendencia) && (w.medida === "minimo" || w.medida === "maximo")) {
        problema(ctx, [...ruta, "medida"], "Comparar y la tendencia sólo sirven al contar");
      }
      break;
    case "barras":
    case "linea":
      revisarMedida(fuente, w, ruta, ctx);
      revisarEjes(fuente, w, ruta, ctx);
      break;
    case "torta":
      revisarMedida(fuente, w, ruta, ctx);
      exigirCampo(fuente, w.categoria, [...ruta, "categoria"], ctx);
      break;
    case "tabla":
      w.columnas.forEach((c, j) => exigirCampo(fuente, c, [...ruta, "columnas", j], ctx));
      if (w.agrupar_por) exigirCampo(fuente, w.agrupar_por, [...ruta, "agrupar_por"], ctx);
      // Agrupada, la tabla sólo ordena por el campo de grupo o por la cantidad.
      if (w.orden && !(w.agrupar_por && w.orden.campo === "cantidad")) {
        exigirCampo(fuente, w.orden.campo, [...ruta, "orden", "campo"], ctx);
      }
      break;
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
