import {
  MAXIMO_CATEGORIAS,
  MAXIMO_SERIES,
  definicionCampo,
  type AgrupacionFecha,
  type FormatoFecha,
  type Origen,
} from "./catalogo";
import type { WidgetBarras, WidgetKpi, WidgetLinea, WidgetTabla, WidgetTorta } from "./esquema";

/**
 * Cálculo de los widgets sobre las filas que devolvió la función SQL. Puro y
 * sin interfaz: el renderizador (`Lienzo.tsx`) sólo dibuja lo que sale de aquí.
 */
export type Valor = string | number | boolean | null;
export type Fila = Record<string, Valor>;

export const SIN_DATO = "(sin dato)";
export const OTROS = "Otros";
const LIMITE_TABLA = 200;
const PORCIONES_POR_OMISION = 6;
/** Más puntos que esto no se rellenan día por día (un eje de varios años). */
const MAXIMO_RELLENO = 400;

/** Día de hoy en Costa Rica (YYYY-MM-DD), el mismo que usan las funciones SQL. */
export function hoyCostaRica(ahora: Date = new Date()): string {
  return new Intl.DateTimeFormat("en-CA", { timeZone: "America/Costa_Rica" }).format(ahora);
}

/**
 * Campos que cuentan los días que faltan para un vencimiento: negativo quiere
 * decir que ya venció. Así guardados sirven para ordenar y filtrar, pero un
 * "-40" en pantalla no se entiende: se muestran en palabras.
 */
export function esDiasParaVencer(campo: string): boolean {
  return campo === "dias_para_vencer";
}

const dias = (n: number) => `${n} ${n === 1 ? "día" : "días"}`;

/** -40 → "Vencido hace 40 días"; 0 → "Vence hoy"; 17 → "Faltan 17 días". */
export function textoDiasParaVencer(n: number): string {
  if (!Number.isFinite(n)) return "—";
  if (n < 0) return `Vencido hace ${dias(-n)}`;
  if (n === 0) return "Vence hoy";
  return `${n === 1 ? "Falta" : "Faltan"} ${dias(n)}`;
}

/** Un rango de días para vencer (filtro "entre"), en palabras. */
export function textoRangoDiasParaVencer(desde: number, hasta: number): string {
  const [a, b] = desde <= hasta ? [desde, hasta] : [hasta, desde];
  if (a === b) return textoDiasParaVencer(a);
  if (b < 0) return `vencidos hace entre ${-b} y ${dias(-a)}`;
  if (b === 0) return `vencidos en los últimos ${dias(-a)} o que vencen hoy`;
  if (a === 0) return `vencen en los próximos ${dias(b)}`;
  if (a > 0) return `vencen dentro de ${a} a ${dias(b)}`;
  return `desde vencidos hace ${dias(-a)} hasta los que vencen en ${dias(b)}`;
}

export function textoValor(valor: Valor | undefined): string {
  if (valor === null || valor === undefined) return "—";
  if (typeof valor === "boolean") return valor ? "Sí" : "No";
  return String(valor);
}

type Medida = WidgetKpi["medida"];

function medir(medida: Medida, campo: string | undefined, filas: Fila[]): number | string | null {
  if (medida === "contar") return filas.length;
  const valores = filas.map((f) => f[campo ?? ""]).filter((v): v is string | number | boolean => v != null);
  if (medida === "contar_distintos") return new Set(valores).size;
  if (valores.length === 0) return null;
  const ordenados = [...valores].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
  return (medida === "minimo" ? ordenados[0] : ordenados[ordenados.length - 1]) as number | string;
}

export function calcularKpi(widget: WidgetKpi, filas: Fila[]): number | string | null {
  return medir(widget.medida, widget.campo, filas);
}

/** Cambio relativo contra el período anterior (0,12 = +12 %); `null` si antes era 0. */
export function variacion(actual: number, anterior: number): number | null {
  return anterior === 0 ? null : (actual - anterior) / anterior;
}

/** El valor del KPI día por día dentro del rango (para la mini línea de tendencia). */
export function calcularTendencia(widget: WidgetKpi, filas: Fila[], desde: string, hasta: string): number[] {
  const porDia = new Map<string, Fila[]>();
  for (const fila of filas) {
    const dia = typeof fila.dia === "string" ? fila.dia : null;
    if (dia) porDia.set(dia, [...(porDia.get(dia) ?? []), fila]);
  }
  const dias = Math.min(diasEntre(desde, hasta), 400);
  return Array.from({ length: dias + 1 }, (_, i) =>
    Number(medir(widget.medida, widget.campo, porDia.get(sumarDias(desde, i)) ?? [])) || 0,
  );
}

// --- Fechas ---------------------------------------------------------------

function aUtc(ymd: string): Date {
  const [a, m, d] = ymd.slice(0, 10).split("-").map(Number);
  return new Date(Date.UTC(a, m - 1, d || 1));
}

function aYmd(fecha: Date): string {
  return fecha.toISOString().slice(0, 10);
}

/** "2026-10-22" más (o menos) `dias` días, cruzando meses y años. */
export function sumarDias(ymd: string, dias: number): string {
  const fecha = aUtc(ymd);
  fecha.setUTCDate(fecha.getUTCDate() + dias);
  return aYmd(fecha);
}

/** Días entre dos fechas YYYY-MM-DD (0 si son la misma). */
export function diasEntre(desde: string, hasta: string): number {
  return Math.round((aUtc(hasta).getTime() - aUtc(desde).getTime()) / 86_400_000);
}

/** Clave ordenable del grupo de fecha: día "2026-10-22", semana = su lunes, mes "2026-10". */
export function grupoFecha(ymd: string, agrupacion: AgrupacionFecha): string {
  if (agrupacion === "mes") return ymd.slice(0, 7);
  if (agrupacion === "dia") return ymd.slice(0, 10);
  const fecha = aUtc(ymd);
  const desdeLunes = (fecha.getUTCDay() + 6) % 7;
  fecha.setUTCDate(fecha.getUTCDate() - desdeLunes);
  return aYmd(fecha);
}

function siguienteGrupo(clave: string, agrupacion: AgrupacionFecha): string {
  if (agrupacion === "mes") {
    const [a, m] = clave.split("-").map(Number);
    return m === 12 ? `${a + 1}-01` : `${a}-${String(m + 1).padStart(2, "0")}`;
  }
  const fecha = aUtc(clave);
  fecha.setUTCDate(fecha.getUTCDate() + (agrupacion === "semana" ? 7 : 1));
  return aYmd(fecha);
}

/** Todos los grupos entre el primero y el último, para que un día sin datos se vea como 0. */
function rellenar(claves: string[], agrupacion: AgrupacionFecha): string[] {
  if (claves.length < 2) return claves;
  const ordenadas = [...claves].sort();
  const todas = [ordenadas[0]];
  const ultima = ordenadas[ordenadas.length - 1];
  while (todas[todas.length - 1] < ultima) {
    if (todas.length > MAXIMO_RELLENO) return ordenadas;
    todas.push(siguienteGrupo(todas[todas.length - 1], agrupacion));
  }
  return todas;
}

const MESES = new Intl.DateTimeFormat("es-CR", { month: "short", year: "numeric", timeZone: "UTC" });
const DIA_MES = new Intl.DateTimeFormat("es-CR", { day: "numeric", month: "short", timeZone: "UTC" });
const FECHA_TEXTO = new Intl.DateTimeFormat("es-CR", { day: "numeric", month: "short", year: "numeric", timeZone: "UTC" });

const sinPunto = (texto: string) => texto.replace(/\./g, "");

/** Fecha completa según el formato del lienzo: 22/10/2026, 2026-10-22 o "22 oct 2026". */
export function formatearFecha(ymd: string, formato: FormatoFecha = "dia_mes_anio"): string {
  if (!/^\d{4}-\d{2}-\d{2}/.test(ymd)) return ymd;
  const [a, m, d] = ymd.slice(0, 10).split("-");
  if (formato === "anio_mes_dia") return `${a}-${m}-${d}`;
  if (formato === "texto") return sinPunto(FECHA_TEXTO.format(aUtc(ymd)));
  return `${d}/${m}/${a}`;
}

/** Fecha corta (sin año), para un eje: "22/10", "10-22" o "22 oct". */
function fechaCorta(ymd: string, formato: FormatoFecha): string {
  const [, m, d] = ymd.split("-");
  if (formato === "anio_mes_dia") return `${m}-${d}`;
  if (formato === "texto") return sinPunto(DIA_MES.format(aUtc(ymd)));
  return `${d}/${m}`;
}

/** Texto corto de un grupo de fecha para el eje: "22/10", "sem. 20/10", "oct 2026". */
export function etiquetaFecha(clave: string, agrupacion: AgrupacionFecha, formato: FormatoFecha = "dia_mes_anio"): string {
  if (clave === SIN_DATO) return clave;
  if (agrupacion === "mes") return sinPunto(MESES.format(aUtc(`${clave}-01`)));
  return agrupacion === "semana" ? `sem. ${fechaCorta(clave, formato)}` : fechaCorta(clave, formato);
}

// --- Series (barras y línea) ----------------------------------------------

export interface Serie {
  nombre: string;
  valores: number[];
  /** La serie que junta lo que no entró ("Otros"): va en gris. */
  otros?: boolean;
}

export interface DatosGrafico {
  /** Claves del eje (para una fecha, ordenables: ver `grupoFecha`). */
  categorias: string[];
  series: Serie[];
  /** Si el eje es de fecha, cómo se agrupó. */
  agrupacion: AgrupacionFecha | null;
}

/** Las claves con más peso primero; las que pasan de `limite` se juntan en "Otros". */
function quedarseConLasMayores(grupos: Map<string, Fila[]>, limite: number, medida: Medida, campo?: string) {
  const ordenadas = [...grupos.keys()].sort(
    (a, b) => Number(medir(medida, campo, grupos.get(b) ?? [])) - Number(medir(medida, campo, grupos.get(a) ?? [])) || a.localeCompare(b),
  );
  // Juntar sólo una clave en "Otros" no ahorra nada: se muestra con su nombre.
  const cuantas = ordenadas.length === limite + 1 ? limite + 1 : limite;
  return new Set(ordenadas.slice(0, cuantas));
}

function agrupar(filas: Fila[], clave: (f: Fila) => string): Map<string, Fila[]> {
  const grupos = new Map<string, Fila[]>();
  for (const fila of filas) {
    const k = clave(fila);
    const grupo = grupos.get(k);
    if (grupo) grupo.push(fila);
    else grupos.set(k, [fila]);
  }
  return grupos;
}

function claveTexto(valor: Valor | undefined): string {
  return valor === null || valor === undefined || valor === "" ? SIN_DATO : textoValor(valor);
}

export function calcularSeries(
  fuente: Origen,
  widget: WidgetBarras | WidgetLinea,
  filas: Fila[],
  nombreSerieUnica = "Cantidad",
): DatosGrafico {
  const esFecha = definicionCampo(fuente, widget.eje_x)?.tipo === "fecha";
  const agrupacion = esFecha ? (widget.agrupar_fecha ?? "dia") : null;
  const claveEje = (f: Fila) => {
    const v = f[widget.eje_x];
    return agrupacion && typeof v === "string" ? grupoFecha(v, agrupacion) : claveTexto(v);
  };

  // Eje: una fecha conserva todos sus grupos (rellenos); lo demás, los mayores + "Otros".
  const porEje = agrupar(filas, claveEje);
  const limite = widget.tipo === "barras" ? (widget.limite ?? MAXIMO_CATEGORIAS) : MAXIMO_CATEGORIAS;
  const ejeVisible = agrupacion ? null : quedarseConLasMayores(porEje, limite, widget.medida, widget.campo);
  const ejeFinal = (f: Fila) => {
    const k = claveEje(f);
    return !ejeVisible || ejeVisible.has(k) ? k : OTROS;
  };

  // Serie: las mayores + "Otros"; sin `serie`, una sola.
  const claveSerie = (f: Fila) => (widget.serie ? claveTexto(f[widget.serie]) : nombreSerieUnica);
  const seriesVisibles = quedarseConLasMayores(agrupar(filas, claveSerie), MAXIMO_SERIES, widget.medida, widget.campo);
  const serieFinal = (f: Fila) => (seriesVisibles.has(claveSerie(f)) ? claveSerie(f) : OTROS);

  const celdas = agrupar(filas, (f) => `${ejeFinal(f)}\u0000${serieFinal(f)}`);
  const valor = (x: string, s: string) => Number(medir(widget.medida, widget.campo, celdas.get(`${x}\u0000${s}`) ?? []));

  let categorias: string[];
  if (agrupacion) {
    const fechas = [...porEje.keys()].filter((k) => k !== SIN_DATO);
    categorias = rellenar(fechas, agrupacion);
    if (porEje.has(SIN_DATO)) categorias.push(SIN_DATO);
  } else {
    const usadas = new Set(filas.map(ejeFinal));
    const total = (x: string) => Number(medir(widget.medida, widget.campo, filas.filter((f) => ejeFinal(f) === x)));
    categorias = [...usadas].filter((x) => x !== OTROS).sort((a, b) => total(b) - total(a) || a.localeCompare(b));
    if (usadas.has(OTROS)) categorias.push(OTROS);
  }

  const totalSerie = (s: string) => Number(medir(widget.medida, widget.campo, filas.filter((f) => serieFinal(f) === s)));
  const nombres = [...new Set(filas.map(serieFinal))];
  const ordenSeries = nombres.filter((s) => s !== OTROS).sort((a, b) => totalSerie(b) - totalSerie(a) || a.localeCompare(b));
  if (nombres.includes(OTROS)) ordenSeries.push(OTROS);

  return {
    categorias,
    agrupacion,
    series: ordenSeries.map((nombre) => ({
      nombre,
      valores: categorias.map((x) => valor(x, nombre)),
      ...(nombre === OTROS ? { otros: true } : {}),
    })),
  };
}

// --- Torta ------------------------------------------------------------------

export interface Porcion {
  nombre: string;
  valor: number;
  otros?: boolean;
}

export function calcularPorciones(widget: WidgetTorta, filas: Fila[]): { porciones: Porcion[]; total: number } {
  const clave = (f: Fila) => claveTexto(f[widget.categoria]);
  const grupos = agrupar(filas, clave);
  const visibles = quedarseConLasMayores(grupos, widget.limite ?? PORCIONES_POR_OMISION, widget.medida, widget.campo);
  const final = agrupar(filas, (f) => (visibles.has(clave(f)) ? clave(f) : OTROS));
  const porciones: Porcion[] = [...final]
    .map(([nombre, grupo]) => ({ nombre, valor: Number(medir(widget.medida, widget.campo, grupo)) }))
    .filter((p) => p.nombre !== OTROS)
    .sort((a, b) => b.valor - a.valor || a.nombre.localeCompare(b.nombre));
  if (final.has(OTROS)) {
    porciones.push({ nombre: OTROS, valor: Number(medir(widget.medida, widget.campo, final.get(OTROS) ?? [])), otros: true });
  }
  // Con "contar distintos" las porciones pueden compartir valores: el total es el de todas las filas.
  return { porciones, total: Number(medir(widget.medida, widget.campo, filas)) };
}

// --- Tabla ------------------------------------------------------------------

export interface TablaCalculada {
  columnas: { id: string; etiqueta: string }[];
  filas: Fila[];
  /** Filas que hubo antes de recortar por el límite. */
  total: number;
}

/** Compara dos valores; con `signo` -1 invierte el orden pero los vacíos siguen al final. */
function comparar(a: Valor | undefined, b: Valor | undefined, signo = 1): number {
  if (a == null && b == null) return 0;
  if (a == null) return 1;
  if (b == null) return -1;
  return signo * (a < b ? -1 : a > b ? 1 : 0);
}

export function calcularTabla(
  fuente: Origen,
  widget: WidgetTabla,
  filas: Fila[],
  formatoFecha: FormatoFecha = "dia_mes_anio",
): TablaCalculada {
  const etiqueta = (id: string) => (id === "cantidad" ? "Cantidad" : (definicionCampo(fuente, id)?.etiqueta ?? id));
  let columnas = widget.columnas;
  let resultado = filas;

  if (widget.agrupar_por) {
    const campo = widget.agrupar_por;
    const cuentas = new Map<Valor, number>();
    for (const fila of filas) cuentas.set(fila[campo], (cuentas.get(fila[campo]) ?? 0) + 1);
    resultado = [...cuentas].map(([valor, cantidad]) => ({ [campo]: valor, cantidad }));
    columnas = [campo, "cantidad"];
    // Agrupada y sin orden pedido: los grupos más grandes primero.
    resultado.sort((a, b) => Number(b.cantidad) - Number(a.cantidad) || comparar(a[campo], b[campo]));
  }

  if (widget.orden) {
    const { campo, dir } = widget.orden;
    const signo = dir === "asc" ? 1 : -1;
    // `sort` es estable: los empates conservan el orden anterior.
    resultado = [...resultado].sort((a, b) => comparar(a[campo], b[campo], signo));
  }

  // Fechas y días para vencer se escriben recién al final: ordenar va sobre el valor real (ISO, número).
  const deFecha = columnas.filter((id) => definicionCampo(fuente, id)?.tipo === "fecha");
  const deDias = columnas.filter(esDiasParaVencer);
  const limite = widget.limite ?? LIMITE_TABLA;
  const visibles = resultado.slice(0, limite).map((fila) => {
    if (deFecha.length === 0 && deDias.length === 0) return fila;
    const copia = { ...fila };
    for (const id of deFecha) if (typeof copia[id] === "string") copia[id] = formatearFecha(copia[id], formatoFecha);
    for (const id of deDias) if (typeof copia[id] === "number") copia[id] = textoDiasParaVencer(copia[id]);
    return copia;
  });
  return {
    columnas: columnas.map((id) => ({ id, etiqueta: etiqueta(id) })),
    filas: visibles,
    total: resultado.length,
  };
}
