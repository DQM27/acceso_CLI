/**
 * Catálogo cerrado del lienzo (plan del asistente, Parte A, etapa 1): lo único
 * que un documento de lienzo puede pedir. Todo lo que no esté aquí se rechaza
 * al validar; el servidor (funciones SQL `lienzo_*`) usa la misma lista blanca.
 */
export type TipoCampo = "texto" | "enumeracion" | "fecha" | "numero" | "booleano";

export const OPERADORES = ["igual", "en", "no_en", "entre", "contiene"] as const;
export type Operador = (typeof OPERADORES)[number];

export interface DefinicionCampo {
  tipo: TipoCampo;
  etiqueta: string;
  /** Valores permitidos, sólo para `enumeracion`. */
  valores?: readonly string[];
  /** Operadores que acepta la función SQL de este campo, si son menos que los de su tipo. */
  operadores?: readonly Operador[];
  /** Sus `valores` tienen un orden propio (lunes a domingo): los gráficos lo respetan en vez de ordenar por tamaño. */
  ordinal?: boolean;
}

export const ESTADOS_PRAIND = ["VIGENTE", "POR_VENCER", "VENCIDA", "SIN_REGISTRO", "NO_REQUIERE"] as const;
export const TIPOS_INGRESO = ["PRAIND", "IN_HOUSE", "POR_CORREO", "SWAT"] as const;
/** En orden: así se ordenan los ejes y las filas (lunes primero, como en Costa Rica). */
export const DIAS_SEMANA = ["Lunes", "Martes", "Miércoles", "Jueves", "Viernes", "Sábado", "Domingo"] as const;

export const CATALOGO = {
  contratistas: {
    etiqueta: "Contratistas",
    campos: {
      nombre: { tipo: "texto", etiqueta: "Nombre", operadores: ["contiene"] },
      empresa: { tipo: "texto", etiqueta: "Empresa" },
      estado_praind: { tipo: "enumeracion", etiqueta: "Estado PRAIND", valores: ESTADOS_PRAIND },
      vencimiento: { tipo: "fecha", etiqueta: "Vencimiento" },
      dias_para_vencer: { tipo: "numero", etiqueta: "Días para vencer" },
      tipo_ingreso: { tipo: "enumeracion", etiqueta: "Tipo de ingreso", valores: TIPOS_INGRESO },
      sitio: { tipo: "texto", etiqueta: "Sitio donde está adentro", operadores: ["igual"] },
      acceso_activo: { tipo: "booleano", etiqueta: "Acceso activo" },
    },
  },
  ingresos: {
    etiqueta: "Ingresos",
    campos: {
      nombre: { tipo: "texto", etiqueta: "Nombre", operadores: ["contiene"] },
      empresa: { tipo: "texto", etiqueta: "Empresa" },
      tipo_ingreso: { tipo: "enumeracion", etiqueta: "Tipo de ingreso", valores: TIPOS_INGRESO },
      sitio: { tipo: "texto", etiqueta: "Sitio", operadores: ["igual"] },
      dia: { tipo: "fecha", etiqueta: "Día" },
      // Los tres que siguen se filtran en el panel, no en la función SQL (ver CAMPOS_FILTRADOS_EN_PANEL).
      dia_semana: { tipo: "enumeracion", etiqueta: "Día de la semana", valores: DIAS_SEMANA, ordinal: true },
      hora: { tipo: "numero", etiqueta: "Hora de entrada" },
      horas_adentro: { tipo: "numero", etiqueta: "Horas adentro" },
    },
  },
} as const satisfies Record<string, { etiqueta: string; campos: Record<string, DefinicionCampo> }>;

/**
 * Campos de ingresos que la función SQL devuelve pero no filtra: el panel los
 * filtra sobre las filas que ya trajo (son pocas: como mucho 92 días).
 */
export const CAMPOS_FILTRADOS_EN_PANEL: readonly string[] = ["dia_semana", "hora", "horas_adentro"];

export type Fuente = keyof typeof CATALOGO;
export const FUENTES = Object.keys(CATALOGO) as [Fuente, ...Fuente[]];
/** Fuente de un lienzo: una del catálogo, o "consulta" (un SELECT del asistente sobre el esquema `ia`). */
export type FuenteLienzo = Fuente | "consulta";

/** Tipos que puede declarar una columna de consulta (las enumeraciones son texto). */
export const TIPOS_COLUMNA = ["texto", "numero", "fecha", "booleano"] as const;
export type TipoColumna = (typeof TIPOS_COLUMNA)[number];

export interface ColumnaConsulta {
  nombre: string;
  tipo: TipoColumna;
  etiqueta?: string;
}

/**
 * De dónde salen los campos de un lienzo: una fuente del catálogo, o las
 * columnas que declara su consulta.
 */
export type Origen = Fuente | { readonly columnas: readonly ColumnaConsulta[] };

/** Operadores que admite cada tipo de campo. */
export const OPERADORES_POR_TIPO: Record<TipoCampo, readonly Operador[]> = {
  texto: ["igual", "en", "no_en", "contiene"],
  enumeracion: ["igual", "en", "no_en"],
  fecha: ["igual", "entre"],
  numero: ["igual", "entre"],
  booleano: ["igual"],
};

/** "sumar" suma un campo de número: para datos que ya vienen calculados (una consulta o un análisis en Python). */
export const MEDIDAS = ["contar", "contar_distintos", "sumar", "minimo", "maximo"] as const;
export type Medida = (typeof MEDIDAS)[number];

export const WIDGETS = ["kpi", "tabla", "barras", "linea", "torta", "calor", "calendario", "libre", "analisis"] as const;

/** Cómo se reparten los paneles: en cuadrícula, o uno solo grande al centro. */
export const DISPOSICIONES = ["cuadricula", "enfoque"] as const;
/** Ancho de un panel dentro de la cuadrícula (de 12 columnas). */
export const ANCHOS = ["cuarto", "tercio", "mitad", "dos_tercios", "completo"] as const;
export type Ancho = (typeof ANCHOS)[number];
/** Agrupar un eje de fecha por día, semana (que empieza el lunes) o mes. */
export const AGRUPACIONES_FECHA = ["dia", "semana", "mes"] as const;
export type AgrupacionFecha = (typeof AGRUPACIONES_FECHA)[number];
/** Cómo se escriben las fechas: 22/10/2026, 2026-10-22 o "22 oct 2026". */
export const FORMATOS_FECHA = ["dia_mes_anio", "anio_mes_dia", "texto"] as const;
export type FormatoFecha = (typeof FORMATOS_FECHA)[number];
/** Color de estado de una tarjeta KPI; siempre acompañado de su texto. */
export const TONOS = ["neutro", "exito", "advertencia", "error"] as const;

/** Tope de categorías en un gráfico: el resto se junta en "Otros". */
export const MAXIMO_CATEGORIAS = 30;
/** Tope de series (colores) en un gráfico: el resto se junta en "Otros". */
export const MAXIMO_SERIES = 7;

export function definicionCampo(origen: Origen, campo: string): DefinicionCampo | undefined {
  if (typeof origen !== "string") {
    const columna = origen.columnas.find((c) => c.nombre === campo);
    return columna && { tipo: columna.tipo, etiqueta: columna.etiqueta ?? columna.nombre };
  }
  const campos: Record<string, DefinicionCampo> = CATALOGO[origen].campos;
  return Object.hasOwn(campos, campo) ? campos[campo] : undefined;
}

/** Etiqueta de la fuente de datos, para el subtítulo del lienzo. */
export function etiquetaOrigen(origen: Origen): string {
  return typeof origen === "string" ? CATALOGO[origen].etiqueta : "Consulta del asistente";
}
