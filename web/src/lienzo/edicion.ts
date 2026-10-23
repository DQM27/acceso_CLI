import { OPERADORES_POR_TIPO, definicionCampo, type AgrupacionFecha, type Origen } from "./catalogo";
import { OTROS, SIN_DATO, formatearFecha, sumarDias } from "./calculo";
import type { Filtro, Lienzo } from "./esquema";

/**
 * Ediciones del documento del lienzo que hace la persona desde la pantalla
 * (clic para profundizar, quitar un filtro). Devuelven un documento NUEVO, que
 * pasa por `validarLienzo` antes de aplicarse: el mismo camino que seguirán las
 * ediciones del modelo.
 */

function ultimoDiaDelMes(mes: string): string {
  const [a, m] = mes.split("-").map(Number);
  return new Date(Date.UTC(a, m, 0)).toISOString().slice(0, 10);
}

/**
 * El filtro que deja sólo una categoría de un gráfico (la barra o porción que
 * se tocó). `null` si no se puede: "Otros", "(sin dato)" o un campo cuya
 * función SQL no acepta "igual" (como el nombre, que sólo busca por texto).
 */
export function filtroParaCategoria(
  fuente: Origen,
  campo: string,
  clave: string,
  agrupacion: AgrupacionFecha | null = null,
): Filtro | null {
  if (clave === OTROS || clave === SIN_DATO) return null;
  const def = definicionCampo(fuente, campo);
  if (!def) return null;
  const operadores = def.operadores ?? OPERADORES_POR_TIPO[def.tipo];
  switch (def.tipo) {
    case "fecha":
      if (agrupacion === "semana") return { campo, op: "entre", valor: [clave, sumarDias(clave, 6)] };
      if (agrupacion === "mes") return { campo, op: "entre", valor: [`${clave}-01`, ultimoDiaDelMes(clave)] };
      return { campo, op: "igual", valor: clave };
    case "booleano":
      return clave === "Sí" || clave === "No" ? { campo, op: "igual", valor: clave === "Sí" } : null;
    case "numero":
      return Number.isFinite(Number(clave)) ? { campo, op: "igual", valor: Number(clave) } : null;
    case "enumeracion":
      return (def.valores ?? []).includes(clave) ? { campo, op: "igual", valor: clave } : null;
    case "texto":
      return operadores.includes("igual") ? { campo, op: "igual", valor: clave } : null;
  }
}

/** El lienzo con `filtro` en vez de cualquier otro filtro sobre el mismo campo. */
export function conFiltro(lienzo: Lienzo, filtro: Filtro): Lienzo {
  return { ...lienzo, filtros: [...lienzo.filtros.filter((f) => f.campo !== filtro.campo), filtro] };
}

export function sinFiltro(lienzo: Lienzo, indice: number): Lienzo {
  return { ...lienzo, filtros: lienzo.filtros.filter((_, i) => i !== indice) };
}

/** "Empresa: ACME", "Estado PRAIND: sin VIGENTE", "Día: 01/10/2026 a 31/10/2026"… para los chips. */
export function describirFiltro(fuente: Origen, filtro: Filtro, formato?: Parameters<typeof formatearFecha>[1]): string {
  const def = definicionCampo(fuente, filtro.campo);
  const texto = (v: unknown) =>
    def?.tipo === "fecha" && typeof v === "string"
      ? formatearFecha(v, formato)
      : typeof v === "boolean"
        ? v
          ? "Sí"
          : "No"
        : String(v);
  const lista = Array.isArray(filtro.valor) ? filtro.valor : [filtro.valor];
  const valores =
    filtro.op === "entre"
      ? `${texto(lista[0])} a ${texto(lista[1])}`
      : filtro.op === "contiene"
        ? `contiene "${texto(lista[0])}"`
        : `${filtro.op === "no_en" ? "sin " : ""}${lista.map(texto).join(", ")}`;
  return `${def?.etiqueta ?? filtro.campo}: ${valores}`;
}
