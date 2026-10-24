import { definicionCampo, type FormatoFecha, type Origen } from "./catalogo";
import {
  OTROS,
  SIN_DATO,
  claveTexto,
  diasEntre,
  etiquetaFecha,
  formatearFecha,
  hoyCostaRica,
  medir,
  sumarDias,
  type Fila,
} from "./calculo";
import type { WidgetCalendario, WidgetCalor } from "./esquema";
import { formatoNumero } from "./graficos/utiles";

/**
 * Cálculo del mapa de calor y del calendario del lienzo. Puro y sin interfaz:
 * los gráficos (`graficos/Calor.tsx`, `graficos/Calendario.tsx`) sólo dibujan.
 *
 * El color de una celda no sale del valor exacto sino de su nivel (1 a 5, 0 si
 * está vacía): cinco tonos de un solo color se distinguen bien, también con
 * daltonismo, y la leyenda dice qué rango cubre cada uno.
 */
export const NIVELES = 5;

/** Nivel de color de un valor (0 = vacío, 1 a 5 = de menos a más). */
export function nivel(valor: number, maximo: number): number {
  if (!(valor > 0) || !(maximo > 0)) return 0;
  return Math.min(NIVELES, Math.max(1, Math.ceil((valor / maximo) * NIVELES)));
}

/** Qué valores cubre cada nivel ("1–4", "5–8"…), para la leyenda. Con conteos, rangos enteros. */
export function rangosDeNiveles(maximo: number): { nivel: number; texto: string }[] {
  if (!(maximo > 0)) return [];
  const entero = Number.isInteger(maximo);
  const rangos: { nivel: number; texto: string }[] = [];
  let desdeAnterior = 0;
  for (let n = 1; n <= NIVELES; n++) {
    const hasta = (maximo * n) / NIVELES;
    const desde = entero ? Math.floor(desdeAnterior) + 1 : desdeAnterior;
    const hastaTexto = entero ? Math.floor(hasta) : Math.round(hasta * 10) / 10;
    desdeAnterior = hasta;
    if (entero && Math.floor(hasta) < desde) continue; // nivel sin ningún entero (máximos chicos)
    const texto = entero && desde === hastaTexto ? formatoNumero(desde) : `${formatoNumero(entero ? desde : Math.round(desde * 10) / 10)}–${formatoNumero(hastaTexto)}`;
    rangos.push({ nivel: n, texto });
  }
  return rangos;
}

// --- Mapa de calor -------------------------------------------------------------

export interface Categoria {
  clave: string;
  etiqueta: string;
}

export interface DatosCalor {
  filas: Categoria[];
  columnas: Categoria[];
  /** valores[fila][columna] */
  valores: number[][];
  maximo: number;
}

const MAXIMO_FILAS_CALOR = 15;
const MAXIMO_COLUMNAS_CALOR = 31;
/** Un eje de números enteros con un rango corto se completa (las horas sin ingresos también se ven). */
const MAXIMO_RELLENO_NUMEROS = 48;
const MAXIMO_RELLENO_FECHAS = 62;

function etiquetaDe(origen: Origen, campo: string, clave: string, formato?: FormatoFecha): string {
  if (clave === SIN_DATO || clave === OTROS) return clave;
  const def = definicionCampo(origen, campo);
  if (def?.tipo === "fecha") return etiquetaFecha(clave, "dia", formato);
  if (def?.tipo === "numero") return campo === "hora" ? `${clave} h` : formatoNumero(Number(clave));
  return clave;
}

/**
 * Las categorías de un eje, en un orden con sentido: los días de la semana de
 * lunes a domingo, las horas y fechas en orden y sin huecos, y los textos de
 * mayor a menor (los que no entran, a "Otros").
 */
function categoriasDe(
  origen: Origen,
  campo: string,
  filas: Fila[],
  medida: (subconjunto: Fila[]) => number,
  maximo: number,
): string[] {
  const def = definicionCampo(origen, campo);
  const presentes = [...new Set(filas.map((f) => claveTexto(f[campo])))];
  const conDato = presentes.filter((c) => c !== SIN_DATO);
  const alFinal = presentes.includes(SIN_DATO) ? [SIN_DATO] : [];

  if (def?.tipo === "enumeracion" && def.valores) {
    const extras = conDato.filter((c) => !def.valores?.includes(c));
    return [...def.valores, ...extras, ...alFinal];
  }
  if (def?.tipo === "numero") {
    const numeros = conDato.map(Number).filter(Number.isFinite).sort((a, b) => a - b);
    const enteros = numeros.every(Number.isInteger);
    if (enteros && numeros.length > 0 && numeros[numeros.length - 1] - numeros[0] <= MAXIMO_RELLENO_NUMEROS) {
      const desde = numeros[0];
      return [...Array.from({ length: numeros[numeros.length - 1] - desde + 1 }, (_, i) => String(desde + i)), ...alFinal];
    }
    return [...numeros.map(String), ...alFinal];
  }
  if (def?.tipo === "fecha") {
    const fechas = conDato.map((c) => c.slice(0, 10)).sort();
    if (fechas.length > 1 && diasEntre(fechas[0], fechas[fechas.length - 1]) <= MAXIMO_RELLENO_FECHAS) {
      return [...Array.from({ length: diasEntre(fechas[0], fechas[fechas.length - 1]) + 1 }, (_, i) => sumarDias(fechas[0], i)), ...alFinal];
    }
    return [...new Set(fechas), ...alFinal];
  }
  // Textos (y sí/no): de mayor a menor; lo que no entra va a "Otros".
  const porValor = conDato
    .map((clave) => ({ clave, valor: medida(filas.filter((f) => claveTexto(f[campo]) === clave)) }))
    .sort((a, b) => b.valor - a.valor || a.clave.localeCompare(b.clave));
  const visibles = porValor.slice(0, maximo).map((c) => c.clave);
  return porValor.length > maximo ? [...visibles, OTROS, ...alFinal] : [...visibles, ...alFinal];
}

export function calcularCalor(origen: Origen, widget: WidgetCalor, filas: Fila[], formato?: FormatoFecha): DatosCalor {
  const medida = (subconjunto: Fila[]) => Number(medir(widget.medida, widget.campo, subconjunto)) || 0;
  const claveFila = categoriasDe(origen, widget.filas, filas, medida, MAXIMO_FILAS_CALOR);
  const claveColumna = categoriasDe(origen, widget.columnas, filas, medida, MAXIMO_COLUMNAS_CALOR);
  const enFila = new Set(claveFila);
  const enColumna = new Set(claveColumna);
  const ubicar = (conjunto: Set<string>, clave: string) => (conjunto.has(clave) ? clave : OTROS);

  const grupos = new Map<string, Fila[]>();
  for (const fila of filas) {
    const clave = `${ubicar(enFila, claveTexto(fila[widget.filas]))}\u0000${ubicar(enColumna, claveTexto(fila[widget.columnas]))}`;
    const grupo = grupos.get(clave);
    if (grupo) grupo.push(fila);
    else grupos.set(clave, [fila]);
  }
  const valores = claveFila.map((f) => claveColumna.map((c) => medida(grupos.get(`${f}\u0000${c}`) ?? [])));
  return {
    filas: claveFila.map((clave) => ({ clave, etiqueta: etiquetaDe(origen, widget.filas, clave, formato) })),
    columnas: claveColumna.map((clave) => ({ clave, etiqueta: etiquetaDe(origen, widget.columnas, clave, formato) })),
    valores,
    maximo: Math.max(0, ...valores.flat()),
  };
}

// --- Calendario ----------------------------------------------------------------

export interface DiaCalendario {
  fecha: string;
  /** Día del mes (1 a 31). */
  numero: number;
  /** 0 lunes … 6 domingo. */
  diaSemana: number;
  /** Semana del mes en que cae (0 = la primera fila). */
  semana: number;
  valor: number;
}

export interface MesCalendario {
  /** "2026-11" */
  clave: string;
  etiqueta: string;
  dias: DiaCalendario[];
  semanas: number;
}

export interface DatosCalendario {
  meses: MesCalendario[];
  maximo: number;
  /** Filas con fecha fuera de los meses que se muestran (el aviso "y N más adelante"). */
  fuera: number;
  /** Filas sin fecha (no se pueden ubicar en el calendario). */
  sinFecha: number;
  hoy: string;
}

const MESES_POR_OMISION = 6;
const NOMBRE_MES = new Intl.DateTimeFormat("es-CR", { month: "long", year: "numeric", timeZone: "UTC" });

function diaDeLaSemana(fecha: string): number {
  const [a, m, d] = fecha.split("-").map(Number);
  return (new Date(Date.UTC(a, m - 1, d)).getUTCDay() + 6) % 7;
}

function mesSiguiente(clave: string): string {
  const [a, m] = clave.split("-").map(Number);
  return m === 12 ? `${a + 1}-01` : `${a}-${String(m + 1).padStart(2, "0")}`;
}

function diasDelMes(clave: string): number {
  const [a, m] = clave.split("-").map(Number);
  return new Date(Date.UTC(a, m, 0)).getUTCDate();
}

export function calcularCalendario(widget: WidgetCalendario, filas: Fila[], hoy = hoyCostaRica()): DatosCalendario {
  const porDia = new Map<string, Fila[]>();
  let sinFecha = 0;
  for (const fila of filas) {
    const v = fila[widget.fecha];
    if (typeof v !== "string" || !/^\d{4}-\d{2}-\d{2}/.test(v)) {
      sinFecha++;
      continue;
    }
    const dia = v.slice(0, 10);
    const grupo = porDia.get(dia);
    if (grupo) grupo.push(fila);
    else porDia.set(dia, [fila]);
  }
  const fechas = [...porDia.keys()].sort();
  if (fechas.length === 0) return { meses: [], maximo: 0, fuera: 0, sinFecha, hoy };

  const cuantos = widget.meses ?? MESES_POR_OMISION;
  const claves: string[] = [fechas[0].slice(0, 7)];
  const ultimo = fechas[fechas.length - 1].slice(0, 7);
  while (claves.length < cuantos && claves[claves.length - 1] < ultimo) claves.push(mesSiguiente(claves[claves.length - 1]));
  const limite = claves[claves.length - 1];

  const medida = (subconjunto: Fila[]) => Number(medir(widget.medida, widget.campo, subconjunto)) || 0;
  let maximo = 0;
  const meses = claves.map((clave): MesCalendario => {
    const primero = diaDeLaSemana(`${clave}-01`);
    const dias = Array.from({ length: diasDelMes(clave) }, (_, i): DiaCalendario => {
      const fecha = `${clave}-${String(i + 1).padStart(2, "0")}`;
      const valor = medida(porDia.get(fecha) ?? []);
      maximo = Math.max(maximo, valor);
      return { fecha, numero: i + 1, diaSemana: (primero + i) % 7, semana: Math.floor((primero + i) / 7), valor };
    });
    return {
      clave,
      etiqueta: NOMBRE_MES.format(new Date(`${clave}-01T00:00:00Z`)),
      dias,
      semanas: dias[dias.length - 1].semana + 1,
    };
  });
  const fuera = fechas.filter((f) => f.slice(0, 7) > limite).reduce((n, f) => n + (porDia.get(f)?.length ?? 0), 0);
  return { meses, maximo, fuera, sinFecha, hoy };
}

/** "lunes 3 de noviembre de 2026" (o en el formato de fecha de la vista), para el tooltip y la tabla. */
export function fechaLarga(fecha: string, formato?: FormatoFecha): string {
  if (formato && formato !== "texto") return formatearFecha(fecha, formato);
  return new Intl.DateTimeFormat("es-CR", { weekday: "long", day: "numeric", month: "long", year: "numeric", timeZone: "UTC" }).format(
    new Date(`${fecha}T00:00:00Z`),
  );
}
