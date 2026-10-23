import type { Lienzo } from "./esquema";

/**
 * Vistas de ejemplo del lienzo, escritas como documentos JSON del catálogo. Sin
 * modelo de IA todavía (etapa 1): sirven para comprobar que el JSON se dibuja
 * bien y, más adelante, serán el punto de partida de lo que edite el chat.
 */
export interface VistaPredefinida {
  id: string;
  titulo: string;
  lienzo: Lienzo;
}

export const VISTAS_PREDEFINIDAS: VistaPredefinida[] = [
  {
    id: "praind-vencidos",
    titulo: "PRAIND vencidos por empresa",
    lienzo: {
      fuente: "contratistas",
      filtros: [{ campo: "estado_praind", op: "igual", valor: "VENCIDA" }],
      widgets: [
        { tipo: "kpi", titulo: "PRAIND vencidos", medida: "contar" },
        { tipo: "kpi", titulo: "Empresas afectadas", medida: "contar_distintos", campo: "empresa" },
        { tipo: "barras", titulo: "Vencidos por empresa", eje_x: "empresa", medida: "contar" },
        {
          tipo: "tabla",
          titulo: "Más recientes primero",
          columnas: ["nombre", "empresa", "vencimiento", "dias_para_vencer"],
          orden: { campo: "dias_para_vencer", dir: "desc" },
          limite: 100,
        },
      ],
    },
  },
  {
    id: "praind-por-vencer",
    titulo: "PRAIND por vencer (30 días)",
    lienzo: {
      fuente: "contratistas",
      filtros: [{ campo: "estado_praind", op: "igual", valor: "POR_VENCER" }],
      widgets: [
        { tipo: "kpi", titulo: "Por vencer", medida: "contar" },
        { tipo: "kpi", titulo: "Vence primero", medida: "minimo", campo: "vencimiento" },
        {
          tipo: "tabla",
          titulo: "Por empresa",
          columnas: ["empresa"],
          agrupar_por: "empresa",
        },
        {
          tipo: "tabla",
          titulo: "Detalle",
          columnas: ["nombre", "empresa", "vencimiento", "dias_para_vencer"],
          orden: { campo: "dias_para_vencer", dir: "asc" },
        },
      ],
    },
  },
  {
    id: "ingresos-por-empresa",
    titulo: "Ingresos por empresa (2 semanas)",
    lienzo: {
      fuente: "ingresos",
      filtros: [],
      widgets: [
        { tipo: "kpi", titulo: "Ingresos", medida: "contar" },
        { tipo: "kpi", titulo: "Personas distintas", medida: "contar_distintos", campo: "nombre" },
        { tipo: "barras", titulo: "Ingresos por día", eje_x: "dia", medida: "contar" },
        { tipo: "barras", titulo: "Ingresos por empresa", eje_x: "empresa", medida: "contar" },
      ],
    },
  },
];
