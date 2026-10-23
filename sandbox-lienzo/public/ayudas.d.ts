// Tipos para las pruebas del panel (web/src/lienzo/sandbox.test.ts). No se publican (.assetsignore).
export interface Tema {
  colores?: string[];
  otros?: string;
  fondo?: string;
  intenso?: string;
  texto?: string;
  textoSuave?: string;
  rejilla?: string;
}

export interface Escala {
  (valor: unknown): number;
  marcas(n?: number): number[];
  invertir(px: number): number;
  dominio: [number, number];
}

export interface EscalaBanda {
  (clave: unknown): number;
  ancho: number;
  paso: number;
  claves: unknown[];
}

export interface Ayudas {
  esc(valor: unknown): string;
  el(etiqueta: string, atributos?: Record<string, unknown>, contenido?: string | string[]): string;
  svg(ancho: number, alto: number, contenido: string | string[], opciones?: { titulo?: string }): string;
  numero(valor: unknown): number;
  suma<T>(datos: T[], f?: (d: T) => unknown): number;
  minimo<T>(datos: T[], f?: (d: T) => unknown): unknown;
  maximo<T>(datos: T[], f?: (d: T) => unknown): unknown;
  agrupar<T>(datos: T[], clave: string | ((d: T) => unknown)): Map<unknown, T[]>;
  contar<T>(datos: T[], clave: string | ((d: T) => unknown), opciones?: { orden?: "n" | "clave" }): { clave: unknown; n: number }[];
  marcas(desde: number, hasta: number, cantidad?: number): number[];
  escalaLineal(dominio: [number, number], rango: [number, number]): Escala;
  escalaBanda(claves: Iterable<unknown>, rango: [number, number], relleno?: number): EscalaBanda;
  escalaFecha(dominio: [string, string], rango: [number, number]): ((fecha: string) => number) & { dias: number };
  formatoNumero(v: unknown, opciones?: { entero?: boolean }): string;
  formatoFecha(ymd: unknown, opciones?: { anio?: boolean; dia?: boolean }): string;
  diasEntre(desde: string, hasta: string): number;
  sumarDias(ymd: string, dias: number): string;
  diaSemana(ymd: unknown): number | null;
  recortar(texto: unknown, maximo?: number): string;
  mezclar(a: string, b: string, t: number): string;
  ejeIzquierdo(escala: Escala, opciones?: { x?: number; ancho?: number; formato?: (v: number) => string; cantidad?: number }): string;
  ejeAbajo(posiciones: [unknown, number][], opciones: { y: number; formato?: (v: unknown) => string; maximoCaracteres?: number }): string;
  leyenda(nombres: string[]): string;
  tip(texto: string): Record<string, unknown>;
  color(i: number): string;
  colorPara(nombre: unknown): string;
  intensidad(t: number): string;
  NOMBRES_DIAS: string[];
  NOMBRES_MESES: string[];
}

export function crearAyudas(tema?: Tema): Ayudas;
