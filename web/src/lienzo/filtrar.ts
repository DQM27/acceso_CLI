import { definicionCampo, type Origen } from "./catalogo";
import type { Fila } from "./calculo";
import type { Filtro } from "./esquema";

/**
 * Filtros del documento aplicados en el panel, sobre las filas de una consulta
 * del asistente (fuente "consulta"). Con las fuentes del catálogo los filtros
 * viajan como parámetros de la función SQL; acá no hay parámetros, así que se
 * aplican a las filas ya traídas. Sirve igual para los chips, el clic en un
 * gráfico para profundizar y el pedido de "quitá las empresas A y B".
 */

/** Minúsculas y sin tildes, para que "Perez" encuentre "PÉREZ". */
function plano(texto: string): string {
  return texto.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

function comoTexto(valor: unknown): string {
  return typeof valor === "string" ? valor : String(valor);
}

function coincide(tipo: string, op: Filtro["op"], valorFila: Fila[string], valor: Filtro["valor"]): boolean {
  if (valorFila === null || valorFila === undefined) return op === "no_en";
  // Las fechas pueden llegar con hora: se compara el día (primeros 10 caracteres).
  const celda = tipo === "fecha" ? comoTexto(valorFila).slice(0, 10) : valorFila;
  const esTexto = tipo === "texto" || tipo === "enumeracion";
  const norm = (v: unknown) => (esTexto ? plano(comoTexto(v)) : v);
  const lista = Array.isArray(valor) ? valor : [valor];

  switch (op) {
    case "igual":
      return norm(celda) === norm(valor);
    case "en":
      return lista.some((v) => norm(celda) === norm(v));
    case "no_en":
      return !lista.some((v) => norm(celda) === norm(v));
    case "contiene":
      return plano(comoTexto(celda)).includes(plano(comoTexto(valor)));
    case "entre": {
      const [desde, hasta] = lista;
      return (celda as string | number) >= (desde as string | number) && (celda as string | number) <= (hasta as string | number);
    }
  }
}

export function aplicarFiltros(origen: Origen, filas: Fila[], filtros: Filtro[]): Fila[] {
  const activos = filtros.flatMap((f) => {
    const def = definicionCampo(origen, f.campo);
    return def ? [{ filtro: f, tipo: def.tipo }] : [];
  });
  if (activos.length === 0) return filas;
  return filas.filter((fila) => activos.every(({ filtro, tipo }) => coincide(tipo, filtro.op, fila[filtro.campo], filtro.valor)));
}
