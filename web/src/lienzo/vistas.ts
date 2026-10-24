import type { Lienzo } from "./esquema";
// Los mismos análisis que el modelo tiene de ejemplo (y que una prueba corre con Pyodide).
import { EJEMPLO_ANALISIS, EJEMPLO_PANDAS } from "../../../supabase/functions/lienzo-ia/instrucciones.ts";

/** Lo real día por día contra lo normal de su día de la semana (pandas); también lo corre la prueba. */
export const REAL_CONTRA_NORMAL = `import pandas as pd

resultado = []
df = pd.DataFrame(datos)
if not df.empty:
    por_dia = df.groupby(pd.to_datetime(df["dia"])).size().asfreq("D", fill_value=0)
    normal = por_dia.groupby(por_dia.index.dayofweek).transform("median")
    resultado = pd.concat([
        pd.DataFrame({"dia": por_dia.index, "cantidad": por_dia.values, "serie": "Real"}),
        pd.DataFrame({"dia": normal.index, "cantidad": normal.values, "serie": "Lo normal"}),
    ])`;

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
        { tipo: "calendario", titulo: "Calendario de vencimientos", fecha: "vencimiento", medida: "contar", meses: 6 },
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
    id: "horas-pico",
    titulo: "Horas pico",
    lienzo: {
      fuente: "ingresos",
      filtros: [],
      widgets: [
        { tipo: "calor", titulo: "Ingresos por día de la semana y hora", filas: "dia_semana", columnas: "hora", medida: "contar" },
        { tipo: "barras", titulo: "Ingresos por hora de entrada", eje_x: "hora", medida: "contar", orientacion: "vertical", ancho: "mitad" },
        { tipo: "barras", titulo: "Ingresos por día de la semana", eje_x: "dia_semana", medida: "contar", orientacion: "vertical", ancho: "mitad" },
      ],
    },
  },
  {
    id: "pronostico-ingresos",
    titulo: "Pronóstico de ingresos (Python)",
    lienzo: {
      fuente: "ingresos",
      filtros: [],
      widgets: [
        {
          tipo: "analisis",
          titulo: "Ingresos por día y pronóstico de la próxima semana",
          descripcion: "Promedio de cada día de la semana, calculado en Python",
          codigo: EJEMPLO_ANALISIS,
          columnas: [
            { nombre: "dia", tipo: "fecha", etiqueta: "Día" },
            { nombre: "cantidad", tipo: "numero", etiqueta: "Ingresos" },
            { nombre: "serie", tipo: "texto", etiqueta: "Serie" },
          ],
          mostrar: { tipo: "linea", eje_x: "dia", medida: "sumar", campo: "cantidad", serie: "serie" },
          ancho: "completo",
        },
      ],
    },
  },
  {
    id: "dias-fuera-de-lo-normal",
    titulo: "Días fuera de lo normal (Python + pandas)",
    lienzo: {
      fuente: "ingresos",
      filtros: [],
      widgets: [
        {
          tipo: "analisis",
          titulo: "Ingresos de cada día contra lo normal",
          descripcion: "Lo normal es la mediana de ese día de la semana, calculada con pandas",
          codigo: REAL_CONTRA_NORMAL,
          columnas: [
            { nombre: "dia", tipo: "fecha", etiqueta: "Día" },
            { nombre: "cantidad", tipo: "numero", etiqueta: "Ingresos" },
            { nombre: "serie", tipo: "texto", etiqueta: "Serie" },
          ],
          mostrar: { tipo: "linea", eje_x: "dia", medida: "sumar", campo: "cantidad", serie: "serie" },
          ancho: "completo",
        },
        {
          tipo: "analisis",
          titulo: "Días fuera de lo normal",
          descripcion: "La mitad más o menos de lo normal, y al menos 3 ingresos de diferencia",
          codigo: EJEMPLO_PANDAS,
          columnas: [
            { nombre: "dia", tipo: "fecha", etiqueta: "Día" },
            { nombre: "cantidad", tipo: "numero", etiqueta: "Ingresos" },
            { nombre: "esperado", tipo: "numero", etiqueta: "Lo normal" },
            { nombre: "diferencia", tipo: "numero", etiqueta: "Diferencia" },
          ],
          mostrar: { tipo: "tabla", columnas: ["dia", "cantidad", "esperado", "diferencia"] },
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
