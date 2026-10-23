import { definicionCampo, type FormatoFecha, type Origen, type TipoCampo } from "./catalogo";
import type { WidgetTabla } from "./esquema";
import {
  diasEntre,
  esDiasParaVencer,
  formatearFecha,
  sumarDias,
  textoDiasParaVencer,
  type Fila,
  type Valor,
} from "./calculo";

/**
 * La tabla del lienzo (widget "tabla") lista para dibujar en la grilla: qué
 * columnas, cómo se escribe y se colorea cada celda, qué columnas llevan barra,
 * y si está agrupada, los grupos (que se abren para ver sus filas) con su
 * cantidad y su mini línea. Puro y sin interfaz: `TablaRica.tsx` sólo dibuja.
 */

const LIMITE_FILAS = 200;
/** Filas que se ven dentro de un grupo abierto; el resto se ve con la búsqueda o al exportar. */
export const LIMITE_POR_GRUPO = 50;
/** Puntos de la mini línea de un grupo (unos 3 meses día por día). */
const MAXIMO_PUNTOS = 92;

export type TipoColumna = TipoCampo | "cantidad";
export type Tono = "exito" | "advertencia" | "error" | "neutro";

export interface ColumnaRica {
  id: string;
  etiqueta: string;
  tipo: TipoColumna;
  /** Mayor valor de la columna (para el largo de las barras); sólo en las que llevan barra. */
  maximoBarra?: number;
}

export interface Grupo {
  /** Clave estable del grupo (para abrirlo y cerrarlo). */
  clave: string;
  valor: Valor;
  cantidad: number;
  filas: Fila[];
  /** Cuántas filas hubo cada día (la mini línea), si la tabla pide tendencia. */
  tendencia?: number[];
}

export interface ModeloTabla {
  /** Columnas de las filas (en una agrupada, las del detalle de cada grupo). */
  columnas: ColumnaRica[];
  /** Sólo en una agrupada: el campo del grupo y su columna "Cantidad". */
  agrupada?: { campo: ColumnaRica; cantidad: ColumnaRica; conTendencia: boolean };
  /** Filas sin agrupar (ya ordenadas y recortadas). */
  filas: Fila[];
  /** Grupos (ya ordenados y recortados). */
  grupos: Grupo[];
  /** Filas o grupos antes de recortar por el límite. */
  total: number;
}

const NOMBRES_ESTADO: Record<string, string> = {
  VIGENTE: "Vigente",
  POR_VENCER: "Por vencer",
  VENCIDA: "Vencida",
  SIN_REGISTRO: "Sin registro",
  NO_REQUIERE: "No requiere",
};
const TONOS_ESTADO: Record<string, Tono> = {
  VIGENTE: "exito",
  POR_VENCER: "advertencia",
  VENCIDA: "error",
  SIN_REGISTRO: "neutro",
  NO_REQUIERE: "neutro",
};
/** Orden de gravedad del estado PRAIND, para ordenar la columna: lo vencido primero. */
const GRAVEDAD_ESTADO = ["VENCIDA", "POR_VENCER", "SIN_REGISTRO", "VIGENTE", "NO_REQUIERE"];
/** Con menos de estos días para vencer, el aviso es de "por vencer" (mismo umbral que el PRAIND). */
const DIAS_AVISO = 30;

const esEstado = (id: string) => id === "estado_praind";

function tipoDe(origen: Origen, id: string): TipoColumna {
  if (id === "cantidad") return "cantidad";
  return definicionCampo(origen, id)?.tipo ?? "texto";
}

function etiquetaDe(origen: Origen, id: string): string {
  return id === "cantidad" ? "Cantidad" : (definicionCampo(origen, id)?.etiqueta ?? id);
}

/** El texto de una celda: fechas en el formato de la vista, días para vencer y estados en palabras. */
export function textoCelda(columna: Pick<ColumnaRica, "id" | "tipo">, valor: Valor | undefined, formatoFecha?: FormatoFecha): string {
  if (valor === null || valor === undefined || valor === "") return "—";
  if (typeof valor === "boolean") return valor ? "Sí" : "No";
  if (esEstado(columna.id) && typeof valor === "string") return NOMBRES_ESTADO[valor] ?? valor;
  if (esDiasParaVencer(columna.id) && typeof valor === "number") return textoDiasParaVencer(valor);
  if (columna.tipo === "fecha" && typeof valor === "string") return formatearFecha(valor.slice(0, 10), formatoFecha);
  if (typeof valor === "number") return valor.toLocaleString("es-CR");
  return String(valor);
}

/** El tono de una celda que se muestra como etiqueta de estado (con ícono y texto), o `null` si es texto común. */
export function tonoCelda(columna: Pick<ColumnaRica, "id">, valor: Valor | undefined): Tono | null {
  if (esEstado(columna.id) && typeof valor === "string") return TONOS_ESTADO[valor] ?? null;
  if (esDiasParaVencer(columna.id) && typeof valor === "number") {
    return valor < 0 ? "error" : valor <= DIAS_AVISO ? "advertencia" : null;
  }
  return null;
}

/** Compara dos valores de una columna; los vacíos siempre al final, también en orden descendente. */
export function compararValores(id: string, a: Valor | undefined, b: Valor | undefined, signo = 1): number {
  const vacio = (v: Valor | undefined) => v === null || v === undefined || v === "";
  if (vacio(a) && vacio(b)) return 0;
  if (vacio(a)) return 1;
  if (vacio(b)) return -1;
  if (esEstado(id)) {
    const rango = (v: Valor | undefined) => {
      const i = GRAVEDAD_ESTADO.indexOf(String(v));
      return i === -1 ? GRAVEDAD_ESTADO.length : i;
    };
    return signo * (rango(a) - rango(b));
  }
  if (typeof a === "number" && typeof b === "number") return signo * (a - b);
  return signo * String(a).localeCompare(String(b), "es", { sensitivity: "base", numeric: true });
}

/** Sin tildes y en minúsculas, para buscar "sanchez" y encontrar "Sánchez". */
function plegar(texto: string): string {
  return texto.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

/** Filas que tienen todas las palabras buscadas en alguna de las columnas (como se ven escritas). */
export function filtrarPorBusqueda(
  filas: Fila[],
  columnas: Pick<ColumnaRica, "id" | "tipo">[],
  busqueda: string,
  formatoFecha?: FormatoFecha,
): Fila[] {
  const palabras = plegar(busqueda).split(/\s+/).filter(Boolean);
  if (palabras.length === 0) return filas;
  return filas.filter((fila) => {
    const texto = plegar(columnas.map((c) => textoCelda(c, fila[c.id], formatoFecha)).join(" "));
    return palabras.every((p) => texto.includes(p));
  });
}

function maximoNumerico(valores: (Valor | undefined)[]): number {
  return Math.max(0, ...valores.map((v) => (typeof v === "number" && Number.isFinite(v) ? v : 0)));
}

/** Cuántas filas hubo cada día entre `desde` y `hasta`, según el campo de fecha. */
function porDia(filas: Fila[], campo: string, desde: string, hasta: string): number[] {
  const cuentas = new Map<string, number>();
  for (const fila of filas) {
    const v = fila[campo];
    if (typeof v === "string") cuentas.set(v.slice(0, 10), (cuentas.get(v.slice(0, 10)) ?? 0) + 1);
  }
  return Array.from({ length: diasEntre(desde, hasta) + 1 }, (_, i) => cuentas.get(sumarDias(desde, i)) ?? 0);
}

/** El rango de días de la tendencia: de la primera a la última fecha, como mucho los últimos 92 días. */
function rangoTendencia(filas: Fila[], campo: string): { desde: string; hasta: string } | null {
  const fechas = filas.map((f) => f[campo]).filter((v): v is string => typeof v === "string").map((v) => v.slice(0, 10)).sort();
  if (fechas.length === 0) return null;
  const hasta = fechas[fechas.length - 1];
  const desde = diasEntre(fechas[0], hasta) >= MAXIMO_PUNTOS ? sumarDias(hasta, -(MAXIMO_PUNTOS - 1)) : fechas[0];
  return diasEntre(desde, hasta) >= 1 ? { desde, hasta } : null;
}

export function modeloTabla(
  origen: Origen,
  widget: WidgetTabla,
  filasFuente: Fila[],
  { busqueda = "", formatoFecha }: { busqueda?: string; formatoFecha?: FormatoFecha } = {},
): ModeloTabla {
  const columna = (id: string): ColumnaRica => ({ id, etiqueta: etiquetaDe(origen, id), tipo: tipoDe(origen, id) });
  const limite = widget.limite ?? LIMITE_FILAS;
  const conBarra = new Set(widget.barras ?? []);
  const campoGrupo = widget.agrupar_por;
  const orden = widget.orden;

  // Las columnas de las filas: en una agrupada, las del detalle (sin el campo del grupo, que ya está en el grupo).
  const columnas = widget.columnas.filter((id) => id !== campoGrupo).map(columna);
  const buscables = campoGrupo ? [columna(campoGrupo), ...columnas] : columnas;
  const filas = filtrarPorBusqueda(filasFuente, buscables, busqueda, formatoFecha);
  const ordenarFilas = (lista: Fila[]) =>
    orden && orden.campo !== "cantidad" && orden.campo !== campoGrupo
      ? [...lista].sort((a, b) => compararValores(orden.campo, a[orden.campo], b[orden.campo], orden.dir === "asc" ? 1 : -1))
      : lista;

  if (!campoGrupo) {
    const ordenadas = ordenarFilas(filas);
    const visibles = ordenadas.slice(0, limite);
    for (const c of columnas) if (conBarra.has(c.id)) c.maximoBarra = maximoNumerico(visibles.map((f) => f[c.id]));
    return { columnas, filas: visibles, grupos: [], total: ordenadas.length };
  }

  const porValor = new Map<string, Grupo>();
  for (const fila of filas) {
    const valor = fila[campoGrupo] ?? null;
    const clave = valor === null ? "\u0000vacio" : String(valor);
    const grupo = porValor.get(clave) ?? { clave, valor, cantidad: 0, filas: [] };
    grupo.cantidad += 1;
    grupo.filas.push(fila);
    porValor.set(clave, grupo);
  }
  let grupos = [...porValor.values()];
  // Por omisión los grupos grandes primero; se puede pedir por cantidad o por el campo del grupo.
  const signo = orden?.dir === "asc" ? 1 : -1;
  if (orden?.campo === campoGrupo) grupos.sort((a, b) => compararValores(campoGrupo, a.valor, b.valor, signo));
  else if (orden?.campo === "cantidad") grupos.sort((a, b) => signo * (a.cantidad - b.cantidad) || compararValores(campoGrupo, a.valor, b.valor));
  else grupos.sort((a, b) => b.cantidad - a.cantidad || compararValores(campoGrupo, a.valor, b.valor));
  const total = grupos.length;
  grupos = grupos.slice(0, limite).map((g) => ({ ...g, filas: ordenarFilas(g.filas) }));

  const rango = widget.tendencia ? rangoTendencia(filas, widget.tendencia) : null;
  if (widget.tendencia && rango) {
    for (const g of grupos) g.tendencia = porDia(g.filas, widget.tendencia, rango.desde, rango.hasta);
  }
  for (const c of columnas) {
    if (conBarra.has(c.id)) c.maximoBarra = maximoNumerico(grupos.flatMap((g) => g.filas.map((f) => f[c.id])));
  }
  return {
    columnas,
    agrupada: {
      campo: columna(campoGrupo),
      // La cantidad de una agrupada siempre lleva su barra: es lo que se compara entre grupos.
      cantidad: { ...columna("cantidad"), maximoBarra: Math.max(0, ...grupos.map((g) => g.cantidad)) },
      conTendencia: rango !== null,
    },
    filas: [],
    grupos,
    total,
  };
}
