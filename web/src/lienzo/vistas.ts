import type { Lienzo } from "./esquema";

/**
 * Vistas de ejemplo del lienzo, escritas como documentos JSON del catálogo. Sin
 * modelo de IA todavía: sirven para comprobar que cada herramienta se dibuja
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
        { tipo: "kpi", titulo: "PRAIND vencidos", medida: "contar", tono: "error" },
        { tipo: "kpi", titulo: "Empresas afectadas", medida: "contar_distintos", campo: "empresa" },
        { tipo: "kpi", titulo: "Venció hace más tiempo", medida: "minimo", campo: "vencimiento" },
        { tipo: "kpi", titulo: "Venció más reciente", medida: "maximo", campo: "vencimiento" },
        { tipo: "barras", titulo: "Vencidos por empresa", eje_x: "empresa", medida: "contar", limite: 12, ancho: "dos_tercios" },
        { tipo: "torta", titulo: "Por tipo de ingreso", categoria: "tipo_ingreso", medida: "contar" },
        {
          tipo: "tabla",
          titulo: "Detalle por empresa",
          columnas: ["empresa", "nombre", "vencimiento", "dias_para_vencer"],
          agrupar_por: "empresa",
          orden: { campo: "dias_para_vencer", dir: "desc" },
          limite: 100,
        },
      ],
    },
  },
  {
    id: "estado-praind",
    titulo: "Estado PRAIND de todos",
    lienzo: {
      fuente: "contratistas",
      filtros: [{ campo: "acceso_activo", op: "igual", valor: true }],
      widgets: [
        { tipo: "torta", titulo: "Estado PRAIND", categoria: "estado_praind", medida: "contar", ancho: "tercio" },
        {
          tipo: "barras",
          titulo: "Estado por tipo de ingreso",
          eje_x: "tipo_ingreso",
          serie: "estado_praind",
          medida: "contar",
          orientacion: "vertical",
          ancho: "dos_tercios",
        },
      ],
    },
  },
  {
    id: "vencimientos-por-mes",
    titulo: "Vencimientos del próximo año",
    lienzo: {
      fuente: "contratistas",
      formato: { fecha: "texto" },
      filtros: [{ campo: "dias_para_vencer", op: "entre", valor: [0, 365] }],
      widgets: [
        { tipo: "kpi", titulo: "Vencen en el año", medida: "contar", tono: "advertencia" },
        { tipo: "kpi", titulo: "El próximo vence", medida: "minimo", campo: "vencimiento" },
        { tipo: "barras", titulo: "Vencimientos por mes", eje_x: "vencimiento", agrupar_fecha: "mes", medida: "contar", ancho: "completo" },
        {
          tipo: "tabla",
          titulo: "Quién vence primero",
          columnas: ["nombre", "empresa", "estado_praind", "vencimiento", "dias_para_vencer"],
          orden: { campo: "dias_para_vencer", dir: "asc" },
        },
      ],
    },
  },
  {
    id: "ingresos-2-semanas",
    titulo: "Ingresos de las últimas 2 semanas",
    lienzo: {
      fuente: "ingresos",
      filtros: [],
      widgets: [
        { tipo: "kpi", titulo: "Ingresos", medida: "contar", comparar: true, tendencia: true },
        { tipo: "kpi", titulo: "Personas distintas", medida: "contar_distintos", campo: "nombre", comparar: true },
        { tipo: "kpi", titulo: "Empresas", medida: "contar_distintos", campo: "empresa", tendencia: true },
        { tipo: "kpi", titulo: "Sitios", medida: "contar_distintos", campo: "sitio" },
        { tipo: "linea", titulo: "Ingresos por día", eje_x: "dia", medida: "contar", area: true, ancho: "dos_tercios" },
        { tipo: "torta", titulo: "Por tipo de ingreso", categoria: "tipo_ingreso", medida: "contar" },
        { tipo: "linea", titulo: "Ingresos por día y sitio", eje_x: "dia", serie: "sitio", medida: "contar" },
        { tipo: "barras", titulo: "Empresas con más ingresos", eje_x: "empresa", medida: "contar", limite: 10 },
        {
          tipo: "tabla",
          titulo: "Ingresos por empresa",
          columnas: ["empresa", "nombre", "sitio", "dia"],
          agrupar_por: "empresa",
          tendencia: "dia",
          orden: { campo: "dia", dir: "desc" },
          ancho: "completo",
        },
      ],
    },
  },
  {
    id: "enfoque-tipos",
    titulo: "Enfoque: tipos de ingreso",
    lienzo: {
      fuente: "ingresos",
      disposicion: "enfoque",
      filtros: [],
      widgets: [{ tipo: "torta", titulo: "Ingresos por tipo, últimas 2 semanas", categoria: "tipo_ingreso", medida: "contar" }],
    },
  },
];
