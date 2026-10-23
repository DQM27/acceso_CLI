import { astMapper, astVisitor, parse, toSql, type Expr, type SelectedColumn, type Statement } from "npm:pgsql-ast-parser@12.0.1";

/**
 * Revisor del SQL que escribe el asistente (fuente "consulta" del lienzo),
 * antes de mandarlo a `public.ia_consultar`. Primera capa de tres (ver la
 * migración `ia_consultas`): la base igual corre en solo lectura y con un rol
 * que sólo lee `ia`, pero acá se rechaza todo lo que no sea una sola consulta
 * de lectura sobre `ia.*` con funciones de una lista blanca.
 *
 * Lo que se ejecuta es el SQL regenerado desde el árbol, no el texto original:
 * lo que llega a la base es exactamente lo que se revisó.
 */

/** Funciones que puede usar una consulta. Todo lo demás (set_config, pg_*, dblink…) se rechaza. */
const FUNCIONES = new Set([
  // Agregados y ventanas.
  "count", "sum", "avg", "min", "max", "string_agg", "array_agg", "bool_and", "bool_or", "every",
  "stddev", "stddev_samp", "stddev_pop", "variance", "var_samp", "var_pop",
  "row_number", "rank", "dense_rank", "percent_rank", "cume_dist", "ntile", "lag", "lead",
  "first_value", "last_value", "nth_value",
  // Condicionales.
  "coalesce", "nullif", "greatest", "least",
  // Fechas.
  "now", "date_trunc", "date_part", "age", "make_date", "make_time", "make_timestamp", "make_interval",
  "to_char", "to_date", "to_timestamp", "timezone", "date", "isfinite", "justify_days", "justify_hours",
  // Texto.
  "lower", "upper", "initcap", "trim", "btrim", "ltrim", "rtrim", "length", "char_length", "substring", "substr",
  "replace", "split_part", "concat", "concat_ws", "left", "right", "position", "strpos", "lpad", "rpad",
  "regexp_replace", "translate", "reverse", "format",
  // Números.
  "round", "floor", "ceil", "ceiling", "abs", "trunc", "mod", "sign", "power", "sqrt",
  // Arreglos y JSON (de columnas de las vistas).
  "array_length", "cardinality", "unnest", "array_to_string", "generate_series",
  "jsonb_array_length", "jsonb_extract_path_text", "jsonb_typeof", "json_array_length", "json_extract_path_text",
]);

export type RevisionSql =
  | { ok: true; sql: string; vistas: string[] }
  | {
    ok: false;
    error: string;
    /** Intentó tocar algo fuera de `ia` o una función prohibida: vale una alerta, no sólo un reintento. */
    peligroso: boolean;
  };

const SENTENCIAS_DE_LECTURA = new Set(["select", "union", "union all", "with", "with recursive", "values"]);

function nombresDeCtes(sentencia: Statement): Set<string> {
  const nombres = new Set<string>();
  const visitante = astVisitor((v) => ({
    with: (w) => {
      for (const b of w.bind) nombres.add(b.alias.name.toLowerCase());
      v.super().with(w);
    },
    withRecursive: (w) => {
      nombres.add(w.alias.name.toLowerCase());
      v.super().withRecursive(w);
    },
  }));
  visitante.statement(sentencia);
  return nombres;
}

/**
 * `GROUP BY 1` / `ORDER BY 1` (número de columna) a la expresión o el alias de
 * esa columna. Hace falta porque `toSql` encierra cada expresión entre
 * paréntesis, y para Postgres `GROUP BY (1)` ya no es "la columna 1" sino la
 * constante 1: cambiaría el resultado en silencio.
 */
function sinNumerosDeColumna(sentencia: Statement): Statement {
  const columnaN = (columnas: SelectedColumn[] | null | undefined, expr: Expr): SelectedColumn | null => {
    if (expr.type !== "integer" || !columnas) return null;
    return columnas[expr.value - 1] ?? null;
  };
  const mapeador = astMapper((m) => ({
    selection: (s) => {
      const columnas = s.columns;
      const groupBy = s.groupBy?.map((e) => columnaN(columnas, e)?.expr ?? e);
      const orderBy = s.orderBy?.map((o) => {
        const columna = columnaN(columnas, o.by);
        if (!columna) return o;
        return { ...o, by: columna.alias ? { type: "ref" as const, name: columna.alias.name } : columna.expr };
      });
      return m.super().selection({ ...s, groupBy, orderBy });
    },
  }));
  return mapeador.statement(sentencia) ?? sentencia;
}

export function revisarSql(texto: string): RevisionSql {
  let sentencias: Statement[];
  try {
    sentencias = parse(texto);
  } catch (error) {
    const detalle = error instanceof Error ? error.message.split("\n")[0] : String(error);
    return { ok: false, error: `No se entiende el SQL: ${detalle}`, peligroso: false };
  }
  if (sentencias.length !== 1) {
    return { ok: false, error: "Tiene que ser una sola consulta", peligroso: sentencias.length > 1 };
  }
  const [sentencia] = sentencias;
  if (!SENTENCIAS_DE_LECTURA.has(sentencia.type)) {
    return { ok: false, error: `Sólo se permiten consultas de lectura (SELECT), no "${sentencia.type}"`, peligroso: true };
  }

  const ctes = nombresDeCtes(sentencia);
  const vistas = new Set<string>();
  const problemas: string[] = [];
  const rechazar = (motivo: string) => problemas.push(motivo);
  const visitante = astVisitor((v) => ({
    tableRef: (t) => {
      const nombre = t.name.toLowerCase();
      if (t.schema === undefined && ctes.has(nombre)) return;
      if (t.schema?.toLowerCase() !== "ia") rechazar(`Sólo se puede leer el esquema ia (no "${t.schema ?? ""}.${t.name}")`);
      else vistas.add(nombre);
    },
    call: (c) => {
      const nombre = c.function.name.toLowerCase();
      if (c.function.schema !== undefined || !FUNCIONES.has(nombre)) rechazar(`Función no permitida: ${c.function.name}`);
      v.super().call(c);
    },
    insert: () => rechazar("No se permite INSERT"),
    update: () => rechazar("No se permite UPDATE"),
    delete: () => rechazar("No se permite DELETE"),
  }));
  visitante.statement(sentencia);
  if (problemas.length > 0) return { ok: false, error: [...new Set(problemas)].join("; "), peligroso: true };
  if (vistas.size === 0) return { ok: false, error: "La consulta no lee ninguna vista de ia", peligroso: false };

  return { ok: true, sql: toSql.statement(sinNumerosDeColumna(sentencia)), vistas: [...vistas].sort() };
}
