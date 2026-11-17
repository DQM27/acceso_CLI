import type { LecturaConteo, PosibleCruce } from "../api";

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
