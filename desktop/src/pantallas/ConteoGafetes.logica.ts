import type { GafeteParaConteo, LecturaConteo, PosibleCruce, ResultadoConteo } from "../api";

/** Cómo se pinta una ficha del tablero.
 * - `pendiente`: se espera y todavía no se contó (al terminar, falta).
 * - `contado`: se esperaba y se contó.
 * - `asignado`: lo tiene puesto alguien adentro; no se espera.
 * - `perdido`: figura perdido; no se espera.
 * - `revisar`: se contó, pero figura asignado o perdido. */
export type EstadoFicha = "pendiente" | "contado" | "asignado" | "perdido" | "revisar";

export function estadoFicha(gafete: GafeteParaConteo, contado: boolean): EstadoFicha {
  const noSeEspera = gafete.estado === "Perdido" || gafete.en_uso_por !== null;
  if (noSeEspera) {
    if (contado) return "revisar";
    return gafete.estado === "Perdido" ? "perdido" : "asignado";
  }
  return contado ? "contado" : "pendiente";
}

/** Texto al pasar el mouse por una ficha. */
export function detalleFicha(gafete: GafeteParaConteo, estado: EstadoFicha): string {
  const n = textoNumero(gafete.numero);
  switch (estado) {
    case "pendiente":
      return `${n}: falta contarlo`;
    case "contado":
      return `${n}: contado (clic para quitarlo)`;
    case "asignado":
      return `${n}: asignado a ${gafete.en_uso_por}`;
    case "perdido":
      return `${n}: perdido`;
    case "revisar":
      return gafete.estado === "Perdido"
        ? `${n}: se contó pero figura perdido (clic para quitarlo)`
        : `${n}: se contó pero figura asignado a ${gafete.en_uso_por} (clic para quitarlo)`;
  }
}

export type FiltroTablero = "todos" | "faltan" | "contados";

export function pasaFiltro(estado: EstadoFicha, filtro: FiltroTablero): boolean {
  switch (filtro) {
    case "todos":
      return true;
    case "faltan":
      return estado === "pendiente";
    case "contados":
      return estado === "contado" || estado === "revisar";
  }
}

/** Gafetes a partir de los cuales el tablero se agrupa por centenas. */
export const AGRUPAR_DESDE = 100;

export interface GrupoTablero {
  /** `null` si no hace falta agrupar (pocos gafetes). */
  titulo: string | null;
  gafetes: GafeteParaConteo[];
}

/** Ordena por número y, con muchos gafetes, agrupa por centenas (001–100,
 * 101–200…). */
export function agruparTablero(gafetes: GafeteParaConteo[]): GrupoTablero[] {
  const ordenados = [...gafetes].sort((a, b) => a.numero - b.numero);
  if (ordenados.length <= AGRUPAR_DESDE) return [{ titulo: null, gafetes: ordenados }];
  const grupos = new Map<number, GafeteParaConteo[]>();
  for (const gafete of ordenados) {
    const centena = Math.floor((gafete.numero - 1) / 100);
    const grupo = grupos.get(centena) ?? [];
    grupo.push(gafete);
    grupos.set(centena, grupo);
  }
  return [...grupos.entries()].map(([centena, lista]) => ({
    titulo: `${String(centena * 100 + 1).padStart(3, "0")}–${String((centena + 1) * 100).padStart(3, "0")}`,
    gafetes: lista,
  }));
}

/** Lo que pide atención en la columna "Revisar", ya redactado. Los
 * asignados o perdidos que forman parte de un posible cruce salen sólo en el
 * cruce, no dos veces. */
export function avisosParaRevisar(resultado: ResultadoConteo): string[] {
  const enCruce = new Set(resultado.posibles_cruces.map((cruce) => cruce.contado.numero));
  return [
    ...resultado.posibles_cruces.map(textoCruce),
    ...resultado.en_uso_contados
      .filter((gafete) => !enCruce.has(gafete.numero))
      .map(
        (gafete) =>
          `El ${textoNumero(gafete.numero)} figura asignado a ${gafete.nombre} y se contó. Revise que esté bien digitado; si lo está, falta registrar su salida.`,
      ),
    ...resultado.perdidos_contados.map(
      (numero) =>
        `El ${textoNumero(numero)} figura perdido y se contó. Revise que esté bien digitado; si lo está, recupérelo desde la lista.`,
    ),
  ];
}

/** Frase de un posible error de digitación al entregar. */
export function textoCruce(cruce: PosibleCruce): string {
  const contado = textoNumero(cruce.contado.numero);
  const faltante = textoNumero(cruce.faltante);
  return `El ${contado} figura asignado a ${cruce.contado.nombre} y se contó; falta el ${faltante}. Revise qué gafete tiene puesto ${cruce.contado.nombre}: pudo digitarse el ${contado} en vez del ${faltante}.`;
}

/** Número de gafete como se ve impreso: dos dígitos mínimo. */
export function textoNumero(numero: number): string {
  return String(numero).padStart(2, "0");
}

/** Lo escrito en el campo como número de gafete, o `null` si no es uno. */
export function parsearNumero(texto: string): number | null {
  const limpio = texto.trim();
  if (!/^\d{1,6}$/.test(limpio)) return null;
  const numero = Number(limpio);
  return numero > 0 ? numero : null;
}

export type NivelAviso = "bien" | "atencion" | "error";

/** Qué se le dice al operador al escribir un número y si queda anotado. */
export interface AvisoNumero {
  nivel: NivelAviso;
  texto: string;
  anotar: boolean;
}

/** Aviso para un número antes de preguntarle al catálogo. `null` si hay que
 * preguntar. */
export function avisoPrevio(texto: string, anotados: number[]): AvisoNumero | null {
  const numero = parsearNumero(texto);
  if (numero === null) {
    return { nivel: "error", texto: "Escriba un número de gafete.", anotar: false };
  }
  if (anotados.includes(numero)) {
    return {
      nivel: "error",
      texto: `El ${textoNumero(numero)} ya lo había colocado.`,
      anotar: false,
    };
  }
  return null;
}

/** Aviso según lo que dice el catálogo del número. Un número que no existe
 * no se anota; uno perdido o asignado a alguien sí (se tiene en la mano), con
 * lo que hay que revisar: primero, que se haya digitado bien. */
export function avisoLectura(numero: number, lectura: LecturaConteo): AvisoNumero {
  const n = textoNumero(numero);
  switch (lectura.tipo) {
    case "esperado":
      return { nivel: "bien", texto: `${n} anotado.`, anotar: true };
    case "no_existe":
      return { nivel: "error", texto: `El ${n} no existe en el catálogo.`, anotar: false };
    case "perdido":
      return {
        nivel: "atencion",
        texto: `El ${n} figura perdido. Quedó anotado: revise que esté bien digitado; si lo es, recupérelo.`,
        anotar: true,
      };
    case "en_uso":
      return {
        nivel: "atencion",
        texto: `El ${n} figura asignado a ${lectura.nombre} (adentro). Quedó anotado: revise que esté bien digitado; si lo es, falta registrar su salida.`,
        anotar: true,
      };
  }
}
