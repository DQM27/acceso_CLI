import { OTROS } from "./calculo";

/**
 * Colores de las series del lienzo. La paleta categórica (8 colores, en
 * `Asistente.css` como `--serie-1` … `--serie-8`, con su versión para tema
 * oscuro) está validada para daltonismo y contraste contra el fondo del panel;
 * el orden de los colores es parte de esa validación, así que se asignan en
 * orden y nunca se inventa un color noveno: lo que no entra va a "Otros", en gris.
 */
export const COLORES_CATEGORICOS = 8;
export const COLOR_OTROS = "var(--serie-otros)";
export const COLOR_UNICO = "var(--serie-unica)";

export function colorDeRanura(ranura: number): string {
  return `var(--serie-${ranura + 1})`;
}

/**
 * El color sigue a la entidad, no a su posición: si un filtro saca a una
 * empresa, las demás conservan su color. Recuerda la ranura de cada nombre
 * mientras viva el lienzo; un nombre nuevo toma la primera ranura libre.
 */
export class RegistroColores {
  private readonly ranuras = new Map<string, number>();

  /** Color de cada nombre (en el mismo orden). "Otros" siempre en gris. */
  asignar(nombres: string[]): string[] {
    const enUso = new Set<number>();
    const pendientes: string[] = [];
    // Primero, los que ya tenían ranura y no chocan entre sí.
    for (const nombre of nombres) {
      if (nombre === OTROS) continue;
      const previa = this.ranuras.get(nombre);
      if (previa !== undefined && !enUso.has(previa)) enUso.add(previa);
      else pendientes.push(nombre);
    }
    for (const nombre of pendientes) {
      let ranura = 0;
      while (enUso.has(ranura) && ranura < COLORES_CATEGORICOS) ranura++;
      if (ranura >= COLORES_CATEGORICOS) continue;
      enUso.add(ranura);
      this.ranuras.set(nombre, ranura);
    }
    return nombres.map((nombre) => {
      if (nombre === OTROS) return COLOR_OTROS;
      const ranura = this.ranuras.get(nombre);
      return ranura === undefined || !enUso.has(ranura) ? COLOR_OTROS : colorDeRanura(ranura);
    });
  }
}
