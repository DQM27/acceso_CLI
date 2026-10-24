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
  TIPOS_COLUMNA,
  TONOS,
  definicionCampo,
  type DefinicionCampo,
  type Origen,
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
const medidaDeGrupo = z.enum(["contar", "contar_distintos", "sumar"]);

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
    /** Agrupada: una fila por grupo con su cantidad, que se abre para ver sus filas (con `columnas`). */
    agrupar_por: campo.optional(),
    orden: ordenEsquema.optional(),
    limite: z.number().int().min(1).max(500).optional(),
    /** Columnas de número que llevan una barra dentro de la celda (en una agrupada, "cantidad" ya la lleva). */
    barras: z.array(campo).min(1).max(4).optional(),
    /** Agrupada: mini línea por grupo, cuántas filas hubo día por día según este campo de fecha. */
    tendencia: campo.optional(),
    ...panel,
  })
  .strict();

/**
 * Mapa de calor: una grilla con un campo en las filas y otro en las columnas
 * (por ejemplo día de la semana × hora: las horas pico), cada celda más oscura
 * cuanto más alto su valor.
 */
const calorEsquema = z
  .object({
    tipo: z.literal("calor"),
    titulo: titulo.optional(),
    filas: campo,
    columnas: campo,
    medida: medidaDeGrupo,
    campo: campo.optional(),
    ...panel,
  })
  .strict();

/**
 * Calendario: los meses con cada día coloreado según su valor (por ejemplo,
 * cuántos PRAIND vencen ese día). `fecha` es el campo de fecha que ubica cada
 * fila en el calendario.
 */
const calendarioEsquema = z
  .object({
    tipo: z.literal("calendario"),
    titulo: titulo.optional(),
    fecha: campo,
    medida: medidaDeGrupo,
    campo: campo.optional(),
    /** Cuántos meses mostrar como máximo, desde el primero con datos (por omisión 6). */
    meses: z.number().int().min(1).max(12).optional(),
    ...panel,
  })
  .strict();

/**
 * Panel "libre": un dibujo a medida para lo que los demás paneles no cubren.
 * `codigo` es el cuerpo de una función JavaScript que corre en el sandbox
 * (lienzo.megabrisas.com, Web Worker sin red ni DOM) con las filas del lienzo y
 * devuelve SVG/HTML. El modelo no ve los datos: escribe el código conociendo
 * sólo las columnas.
 */
const libreEsquema = z
  .object({
    tipo: z.literal("libre"),
    titulo,
    /** Qué muestra, en una frase (accesibilidad y tooltip del marco). */
    descripcion: z.string().min(1).max(200).optional(),
    codigo: z.string().min(1).max(12000),
    ...panel,
  })
  .strict();

/** Una columna que devuelve la consulta (o un análisis): el nombre exacto y su tipo. */
const columnaEsquema = z
  .object({
    nombre: z.string().regex(/^[a-z_][a-z0-9_]{0,39}$/),
    tipo: z.enum(TIPOS_COLUMNA),
    etiqueta: z.string().min(1).max(40).optional(),
  })
  .strict();

/** Los paneles que puede usar un análisis para mostrar su resultado. */
const panelDeResultado = z.discriminatedUnion("tipo", [
  kpiEsquema,
  barrasEsquema,
  lineaEsquema,
  tortaEsquema,
  tablaEsquema,
  calorEsquema,
  calendarioEsquema,
]);

/**
 * Panel "analisis": un cálculo en Python (pronósticos, estadística, días fuera de
 * lo normal) que corre en el sandbox (lienzo.megabrisas.com, Pyodide en un Web
 * Worker sin red) con las filas del lienzo. `codigo` deja en `resultado` una
 * lista de filas con las `columnas` declaradas, y `mostrar` es el panel común
 * (kpi, barras, línea, torta o tabla) que las dibuja. El modelo no ve los datos.
 */
const analisisEsquema = z
  .object({
    tipo: z.literal("analisis"),
    titulo,
    /** Qué calcula, en una frase. */
    descripcion: z.string().min(1).max(200).optional(),
    codigo: z.string().min(1).max(12000),
    columnas: z.array(columnaEsquema).min(1).max(20),
    mostrar: panelDeResultado,
    ...panel,
  })
  .strict();

const widgetEsquema = z.discriminatedUnion("tipo", [
  kpiEsquema,
  barrasEsquema,
  lineaEsquema,
  tortaEsquema,
  tablaEsquema,
  calorEsquema,
  calendarioEsquema,
  libreEsquema,
  analisisEsquema,
]);

/**
 * Un SELECT sobre el esquema `ia` (fuente "consulta"). Acá sólo se valida la
 * forma; el SQL lo revisa el parser de las Edge Functions antes de ejecutarlo.
 */
const consultaEsquema = z
  .object({
    sql: z.string().min(1).max(4000),
    columnas: z.array(columnaEsquema).min(1).max(30),
  })
  .strict();

const lienzoBase = z
  .object({
    fuente: z.enum([...FUENTES, "consulta"]),
    consulta: consultaEsquema.optional(),
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
export type WidgetCalor = z.infer<typeof calorEsquema>;
export type WidgetCalendario = z.infer<typeof calendarioEsquema>;
export type WidgetLibre = z.infer<typeof libreEsquema>;
export type WidgetAnalisis = z.infer<typeof analisisEsquema>;
/** Un panel común (el que muestra el resultado de un análisis). */
export type WidgetComun = z.infer<typeof panelDeResultado>;
export type Lienzo = z.infer<typeof lienzoBase>;
export type Consulta = z.infer<typeof consultaEsquema>;

/** De dónde salen los campos del lienzo: su fuente del catálogo o las columnas de su consulta. */
export function origenDe(lienzo: Pick<Lienzo, "fuente" | "consulta">): Origen {
  if (lienzo.fuente !== "consulta") return lienzo.fuente;
  return { columnas: lienzo.consulta?.columnas ?? [] };
}

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

function revisarFiltro(fuente: Origen, filtro: Filtro, ruta: Ruta, ctx: z.RefinementCtx) {
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
function exigirCampo(fuente: Origen, nombre: string, ruta: Ruta, ctx: z.RefinementCtx) {
  const def = definicionCampo(fuente, nombre);
  if (!def) problema(ctx, ruta, `Campo "${nombre}" fuera del catálogo`);
  return def;
}

function revisarMedida(
  fuente: Origen,
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
  if (w.campo && w.medida === "sumar") {
    const tipo = definicionCampo(fuente, w.campo)?.tipo;
    if (tipo && tipo !== "numero") problema(ctx, [...ruta, "campo"], '"sumar" sólo sirve con un número');
  }
}

/** Eje y serie de un gráfico de barras o de línea. */
function revisarEjes(fuente: Origen, w: WidgetBarras | WidgetLinea, ruta: Ruta, ctx: z.RefinementCtx) {
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

function revisarWidget(fuente: Origen, w: Widget, ruta: Ruta, ctx: z.RefinementCtx) {
  switch (w.tipo) {
    case "kpi":
      revisarMedida(fuente, w, ruta, ctx);
      // Comparar y la tendencia piden el período anterior por día: sólo la fuente de ingresos lo trae.
      if ((w.comparar || w.tendencia) && fuente !== "ingresos") {
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
      w.barras?.forEach((b, j) => {
        const r = [...ruta, "barras", j];
        if (w.agrupar_por && b === "cantidad") return;
        const def = exigirCampo(fuente, b, r, ctx);
        if (def && def.tipo !== "numero") problema(ctx, r, `"${b}" no es un número: no lleva barra`);
        else if (def && !w.columnas.includes(b)) problema(ctx, r, `"${b}" tiene que estar en las columnas`);
      });
      if (w.tendencia) {
        const r = [...ruta, "tendencia"];
        const def = exigirCampo(fuente, w.tendencia, r, ctx);
        if (!w.agrupar_por) problema(ctx, r, "La tendencia por fila sólo va en una tabla agrupada");
        else if (def && def.tipo !== "fecha") problema(ctx, r, "La tendencia necesita un campo de fecha");
      }
      break;
    case "calor": {
      revisarMedida(fuente, w, ruta, ctx);
      const filas = exigirCampo(fuente, w.filas, [...ruta, "filas"], ctx);
      exigirCampo(fuente, w.columnas, [...ruta, "columnas"], ctx);
      if (filas && w.filas === w.columnas) problema(ctx, [...ruta, "columnas"], "Filas y columnas tienen que ser campos distintos");
      break;
    }
    case "calendario": {
      revisarMedida(fuente, w, ruta, ctx);
      const fecha = exigirCampo(fuente, w.fecha, [...ruta, "fecha"], ctx);
      if (fecha && fecha.tipo !== "fecha") problema(ctx, [...ruta, "fecha"], "El calendario necesita un campo de fecha");
      break;
    }
    case "analisis": {
      // El panel que muestra el resultado se revisa contra las columnas que declara el análisis.
      const nombres = w.columnas.map((c) => c.nombre);
      if (new Set(nombres).size !== nombres.length) problema(ctx, [...ruta, "columnas"], "Columnas repetidas");
      revisarWidget({ columnas: w.columnas }, w.mostrar, [...ruta, "mostrar"], ctx);
      break;
    }
  }
}

export const lienzoEsquema = lienzoBase.superRefine((lienzo, ctx) => {
  if (lienzo.fuente === "consulta") {
    if (!lienzo.consulta) return problema(ctx, ["consulta"], 'La fuente "consulta" necesita su consulta');
    const nombres = lienzo.consulta.columnas.map((c) => c.nombre);
    if (new Set(nombres).size !== nombres.length) problema(ctx, ["consulta", "columnas"], "Columnas repetidas");
  } else if (lienzo.consulta) {
    problema(ctx, ["consulta"], 'Sólo la fuente "consulta" lleva consulta');
  }
  const origen = origenDe(lienzo);
  lienzo.filtros.forEach((f, i) => revisarFiltro(origen, f, ["filtros", i], ctx));
  lienzo.widgets.forEach((w, i) => revisarWidget(origen, w, ["widgets", i], ctx));
});

/**
 * La forma del documento en JSON Schema, para describírsela al modelo (la
 * herramienta `editar_lienzo` de `lienzo-ia`). Sale del mismo esquema zod; las
 * reglas cruzadas (campo de la fuente, operador del tipo) las explican las
 * instrucciones y las hace cumplir `validarLienzo`.
 */
export function esquemaJsonDelLienzo(): Record<string, unknown> {
  const { $schema: _descartado, ...esquema } = z.toJSONSchema(lienzoBase) as Record<string, unknown>;
  return esquema;
}

export type ResultadoValidacion = { ok: true; lienzo: Lienzo } | { ok: false; errores: string[] };

export function validarLienzo(entrada: unknown): ResultadoValidacion {
  const resultado = lienzoEsquema.safeParse(entrada);
  if (resultado.success) return { ok: true, lienzo: resultado.data };
  return {
    ok: false,
    errores: resultado.error.issues.map((e) => `${e.path.join(".") || "(raíz)"}: ${e.message}`),
  };
}
