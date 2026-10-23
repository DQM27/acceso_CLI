/**
 * Catálogo cerrado del lienzo (plan del asistente, Parte A, etapa 1): lo único
 * que un documento de lienzo puede pedir. Todo lo que no esté aquí se rechaza
 * al validar; el servidor (funciones SQL `lienzo_*`) usa la misma lista blanca.
 */
export type TipoCampo = "texto" | "enumeracion" | "fecha" | "numero" | "booleano";

export interface DefinicionCampo {
  tipo: TipoCampo;
  etiqueta: string;
  /** Valores permitidos, sólo para `enumeracion`. */
  valores?: readonly string[];
  /** Operadores que acepta la función SQL de este campo, si son menos que los de su tipo. */
  operadores?: readonly Operador[];
}

export const ESTADOS_PRAIND = ["VIGENTE", "POR_VENCER", "VENCIDA", "SIN_REGISTRO", "NO_REQUIERE"] as const;
export const TIPOS_INGRESO = ["PRAIND", "IN_HOUSE", "POR_CORREO", "SWAT"] as const;

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
    },
  },
} as const satisfies Record<string, { etiqueta: string; campos: Record<string, DefinicionCampo> }>;

export type Fuente = keyof typeof CATALOGO;
export const FUENTES = Object.keys(CATALOGO) as [Fuente, ...Fuente[]];

export const OPERADORES = ["igual", "en", "no_en", "entre", "contiene"] as const;
export type Operador = (typeof OPERADORES)[number];

/** Operadores que admite cada tipo de campo. */
export const OPERADORES_POR_TIPO: Record<TipoCampo, readonly Operador[]> = {
  texto: ["igual", "en", "no_en", "contiene"],
  enumeracion: ["igual", "en", "no_en"],
  fecha: ["igual", "entre"],
  numero: ["igual", "entre"],
  booleano: ["igual"],
};

export const MEDIDAS = ["contar", "contar_distintos", "minimo", "maximo"] as const;
export type Medida = (typeof MEDIDAS)[number];

export const WIDGETS = ["kpi", "tabla", "barras"] as const;

export function definicionCampo(fuente: Fuente, campo: string): DefinicionCampo | undefined {
  const campos: Record<string, DefinicionCampo> = CATALOGO[fuente].campos;
  return Object.hasOwn(campos, campo) ? campos[campo] : undefined;
}
