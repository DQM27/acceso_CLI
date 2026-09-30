import { inicioDiaCostaRicaUtc, inicioDiaSiguienteCostaRicaUtc } from "../tiempo";

/**
 * Traduce el modelo de filtros por columna de AG Grid (`api.getFilterModel()`,
 * el mismo que arma la fila de filtros de la grilla) a condiciones que entiende
 * PostgREST sobre la vista `panel_movimientos` -- así los filtros de la
 * grilla trabajan sobre TODO el historial en el servidor, con las mismas
 * opciones que en escritorio (contiene, empieza con, mayor que, entre...).
 *
 * Cada columna produce UNA expresión para `.or(...)` (con `and(...)`/`or(...)`
 * anidados si la columna tiene dos condiciones); entre columnas rige el AND
 * porque cada expresión se aplica como un `.or()` propio.
 */

export type ModeloFiltros = Record<string, unknown>;

type TipoFiltro = "texto" | "numero" | "fecha";

interface ColumnaFiltrable {
  tipo: TipoFiltro;
  /** Columna de `panel_movimientos` contra la que se compara. Para texto es la
   * versión sin tildes ni mayúsculas (`*_p`), salvo las de texto plano. */
  columna: string;
  /** La columna también está dentro de `texto_busqueda` (indexada con
   * trigramas, ver la migración `historial_busqueda_indexada`): su filtro
   * suma un prefiltro contra ese índice para no recorrer todo el período. */
  enTextoBusqueda?: boolean;
}

/** `colId` de la grilla → columna del servidor. Las de texto van contra su
 * versión plegada (`plegar_texto`); las fechas contra el instante. */
export const COLUMNAS_FILTRABLES: Record<string, ColumnaFiltrable> = {
  sitio_nombre: { tipo: "texto", columna: "unidad_p" },
  contratista_cedula: { tipo: "texto", columna: "cedula_p", enTextoBusqueda: true },
  contratista_nombre: { tipo: "texto", columna: "nombre_p", enTextoBusqueda: true },
  empresa_nombre: { tipo: "texto", columna: "empresa_p", enTextoBusqueda: true },
  dispositivo_entrada_tipo: { tipo: "texto", columna: "dispositivo_entrada_tipo" },
  tipo_ingreso: { tipo: "texto", columna: "tipo_p" },
  medio_ingreso: { tipo: "texto", columna: "medio_p" },
  gafete_numero: { tipo: "numero", columna: "gafete_numero" },
  fecha_ingreso: { tipo: "fecha", columna: "hora_entrada" },
  hora_ingreso: { tipo: "texto", columna: "hora_entrada_txt" },
  fecha_salida: { tipo: "fecha", columna: "hora_salida" },
  hora_salida: { tipo: "texto", columna: "hora_salida_txt" },
  usuario_entrada_nombre: { tipo: "texto", columna: "usuario_entrada_p" },
  usuario_salida_nombre: { tipo: "texto", columna: "usuario_salida_p" },
};

/** Igual que `public.plegar_texto` en la base: sin tildes y en minúsculas
 * (la ñ queda como n, igual que `unaccent`). */
export function plegarTexto(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
}

/** Entre comillas para la sintaxis de PostgREST (`col.op."valor"`): así una
 * coma o un paréntesis del valor no se confunden con la sintaxis. */
function entreComillas(valor: string): string {
  return `"${valor.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

/** Texto escapado para `like`/`ilike`: `\`, `%` y `_` literales (una cédula o un
 * nombre nunca actúan como comodines). Un `*` del usuario se cambia por `_`
 * porque PostgREST usa `*` como comodín y no admite escaparlo. */
function escaparLike(texto: string): string {
  return texto.replace(/\\/g, "\\\\").replace(/%/g, "\\%").replace(/_/g, "\\_").replace(/\*/g, "_");
}

function hoja(columna: string, operador: string, valor: string): string {
  return `${columna}.${operador}.${entreComillas(valor)}`;
}

interface CondicionAg {
  type?: string;
  filter?: string | number | null;
  filterTo?: string | number | null;
  dateFrom?: string | null;
  dateTo?: string | null;
}

function condicionTexto(columna: string, { type, filter }: CondicionAg): string | null {
  if (type === "blank") return `or(${columna}.is.null,${hoja(columna, "eq", "")})`;
  if (type === "notBlank") return `and(${columna}.not.is.null,${hoja(columna, "neq", "")})`;
  if (filter == null || String(filter).trim() === "") return null;
  const texto = plegarTexto(String(filter));
  const like = escaparLike(texto);
  switch (type) {
    case "equals":
      return hoja(columna, "eq", texto);
    case "notEqual":
      return `or(${columna}.is.null,${hoja(columna, "neq", texto)})`;
    case "startsWith":
      return hoja(columna, "ilike", `${like}*`);
    case "endsWith":
      return hoja(columna, "ilike", `*${like}`);
    case "notContains":
      return `or(${columna}.is.null,${hoja(columna, "not.ilike", `*${like}*`)})`;
    case "contains":
    default: {
      // Igual que en escritorio: "carlos sa" exige ambas palabras, en cualquier orden.
      const palabras = texto.split(/\s+/).filter((palabra) => palabra.length > 0);
      if (palabras.length === 0) return null;
      const condiciones = palabras.map((palabra) => hoja(columna, "ilike", `*${escaparLike(palabra)}*`));
      return condiciones.length === 1 ? condiciones[0] : `and(${condiciones.join(",")})`;
    }
  }
}

function condicionNumero(columna: string, { type, filter, filterTo }: CondicionAg): string | null {
  if (type === "blank") return `${columna}.is.null`;
  if (type === "notBlank") return `${columna}.not.is.null`;
  const numero = filter == null || filter === "" ? NaN : Number(filter);
  if (Number.isNaN(numero)) return null;
  switch (type) {
    case "notEqual":
      return `or(${columna}.is.null,${columna}.neq.${numero})`;
    case "greaterThan":
      return `${columna}.gt.${numero}`;
    case "greaterThanOrEqual":
      return `${columna}.gte.${numero}`;
    case "lessThan":
      return `${columna}.lt.${numero}`;
    case "lessThanOrEqual":
      return `${columna}.lte.${numero}`;
    case "inRange": {
      const hasta = filterTo == null || filterTo === "" ? NaN : Number(filterTo);
      if (Number.isNaN(hasta)) return null;
      // Mismo criterio que AG Grid por defecto: los extremos no cuentan.
      return `and(${columna}.gt.${numero},${columna}.lt.${hasta})`;
    }
    case "equals":
    default:
      return `${columna}.eq.${numero}`;
  }
}

/** "2026-09-09 00:00:00" (lo que arma AG Grid) o "2026-09-09" → "2026-09-09". */
function soloDia(valor: string | null | undefined): string | null {
  const dia = valor?.slice(0, 10);
  return dia && /^\d{4}-\d{2}-\d{2}$/.test(dia) ? dia : null;
}

/** Fechas por día calendario de Costa Rica, con los mismos cortes que el
 * selector de rango (`inicioDiaCostaRicaUtc`): el día empieza a las 00:00 -06:00
 * y "hasta" es el inicio del día siguiente, exclusivo. */
function condicionFecha(columna: string, { type, dateFrom, dateTo }: CondicionAg): string | null {
  if (type === "blank") return `${columna}.is.null`;
  if (type === "notBlank") return `${columna}.not.is.null`;
  const dia = soloDia(dateFrom);
  if (!dia) return null;
  const inicio = inicioDiaCostaRicaUtc(dia);
  const siguiente = inicioDiaSiguienteCostaRicaUtc(dia);
  switch (type) {
    case "notEqual":
      return `or(${hoja(columna, "lt", inicio)},${hoja(columna, "gte", siguiente)})`;
    case "greaterThan":
      return hoja(columna, "gte", siguiente);
    case "lessThan":
      return hoja(columna, "lt", inicio);
    case "inRange": {
      const diaHasta = soloDia(dateTo);
      if (!diaHasta) return null;
      // Extremos excluidos, como AG Grid por defecto: después de "desde" y antes de "hasta".
      return `and(${hoja(columna, "gte", siguiente)},${hoja(columna, "lt", inicioDiaCostaRicaUtc(diaHasta))})`;
    }
    case "equals":
    default:
      return `and(${hoja(columna, "gte", inicio)},${hoja(columna, "lt", siguiente)})`;
  }
}

/** Tipos de filtro de texto cuyo valor aparece sí o sí dentro del texto de la
 * columna: sólo esos admiten el prefiltro contra `texto_busqueda`. */
const TIPOS_CON_PREFILTRO = new Set(["contains", "equals", "startsWith", "endsWith"]);

/** Prefiltro con índice: cada palabra del valor debe aparecer en
 * `texto_busqueda`. Es una condición más amplia que la de la columna (nunca
 * descarta una fila que la columna aceptaría); sólo acota lo que Postgres
 * revisa. Sólo para una condición única: con dos unidas por OR no sirve. */
function prefiltrosTextoBusqueda(condiciones: CondicionAg[]): string[] {
  if (condiciones.length !== 1) return [];
  const { type = "contains", filter } = condiciones[0];
  if (!TIPOS_CON_PREFILTRO.has(type) || filter == null) return [];
  return plegarTexto(String(filter))
    .split(/\s+/)
    .filter((palabra) => palabra.length > 0)
    .map((palabra) => hoja("texto_busqueda", "like", `*${escaparLike(palabra)}*`));
}

function condicion(tipo: TipoFiltro, columna: string, datos: CondicionAg): string | null {
  if (tipo === "numero") return condicionNumero(columna, datos);
  if (tipo === "fecha") return condicionFecha(columna, datos);
  return condicionTexto(columna, datos);
}

interface FiltroAg extends CondicionAg {
  operator?: "AND" | "OR";
  conditions?: CondicionAg[];
  condition1?: CondicionAg;
  condition2?: CondicionAg;
}

/** Una expresión de `.or(...)` por cada columna filtrada (las columnas que la
 * grilla conoce y que tienen un filtro con valor). */
export function expresionesDeFiltros(modelo: ModeloFiltros | undefined): string[] {
  if (!modelo) return [];
  const expresiones: string[] = [];
  for (const [colId, valor] of Object.entries(modelo)) {
    const config = COLUMNAS_FILTRABLES[colId];
    if (!config || typeof valor !== "object" || valor === null) continue;
    const filtro = valor as FiltroAg;
    const condiciones = filtro.conditions ?? [filtro.condition1, filtro.condition2].filter((c) => c !== undefined);
    const efectivas = condiciones.length > 0 ? condiciones : [filtro];
    const expresionesColumna = efectivas
      .map((datos) => condicion(config.tipo, config.columna, datos))
      .filter((expresion): expresion is string => expresion !== null);
    if (expresionesColumna.length === 0) continue;
    expresiones.push(
      expresionesColumna.length === 1
        ? expresionesColumna[0]
        : `${filtro.operator === "OR" ? "or" : "and"}(${expresionesColumna.join(",")})`,
    );
    if (config.enTextoBusqueda) expresiones.push(...prefiltrosTextoBusqueda(efectivas));
  }
  return expresiones;
}
