import { definicionCampo, type Fuente } from "./catalogo";
import type { WidgetBarras, WidgetKpi, WidgetTabla } from "./esquema";

/**
 * Cálculo de los widgets sobre las filas que devolvió la función SQL. Puro y
 * sin interfaz: el renderizador (`Lienzo.tsx`) sólo dibuja lo que sale de aquí.
 */
export type Valor = string | number | boolean | null;
export type Fila = Record<string, Valor>;

const SIN_DATO = "(sin dato)";
/** Barras: más categorías que esto no se leen en una pantalla. */
export const MAXIMO_BARRAS = 30;
const LIMITE_TABLA = 200;

export function textoValor(valor: Valor | undefined): string {
  if (valor === null || valor === undefined) return "—";
  if (typeof valor === "boolean") return valor ? "Sí" : "No";
  return String(valor);
}

function medir(
  medida: WidgetKpi["medida"] | WidgetBarras["medida"],
  campo: string | undefined,
  filas: Fila[],
): number | string | null {
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

export interface Barra {
  etiqueta: string;
  valor: number;
}

export function calcularBarras(fuente: Fuente, widget: WidgetBarras, filas: Fila[]): Barra[] {
  const grupos = new Map<string, Fila[]>();
  for (const fila of filas) {
    const clave = textoValor(fila[widget.eje_x]) === "—" ? SIN_DATO : textoValor(fila[widget.eje_x]);
    grupos.set(clave, [...(grupos.get(clave) ?? []), fila]);
  }
  const barras = [...grupos].map(([etiqueta, filasGrupo]) => ({
    etiqueta,
    valor: Number(medir(widget.medida, widget.campo, filasGrupo)),
  }));
  // Una fecha se lee en orden cronológico; lo demás, de mayor a menor.
  const porFecha = definicionCampo(fuente, widget.eje_x)?.tipo === "fecha";
  barras.sort((a, b) => (porFecha ? a.etiqueta.localeCompare(b.etiqueta) : b.valor - a.valor || a.etiqueta.localeCompare(b.etiqueta)));
  return barras.slice(0, MAXIMO_BARRAS);
}

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

export function calcularTabla(fuente: Fuente, widget: WidgetTabla, filas: Fila[]): TablaCalculada {
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

  const limite = widget.limite ?? LIMITE_TABLA;
  return {
    columnas: columnas.map((id) => ({ id, etiqueta: etiqueta(id) })),
    filas: resultado.slice(0, limite),
    total: resultado.length,
  };
}
