import type { LecturaConteo } from "../api";

/** Número de gafete como se ve en la mesa: dos dígitos mínimo. */
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
 * no se anota; uno perdido o puesto en alguien sí (está en la mesa), con el
 * aviso de lo que hay que corregir. */
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
        texto: `El ${n} figura perdido. Quedó anotado: si está en la mesa, recupérelo.`,
        anotar: true,
      };
    case "en_uso":
      return {
        nivel: "atencion",
        texto: `El ${n} lo tiene ${lectura.nombre} (adentro). Quedó anotado: falta registrar su salida.`,
        anotar: true,
      };
  }
}
