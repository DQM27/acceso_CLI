/**
 * Filtros de entrada mientras se escribe (no son reglas: las reglas de
 * cédula y nombre las aplica el núcleo, `domain::contratista`).
 */

/** Sanitiza en vivo mientras se escribe (`onChange`) — deja sólo dígitos.
 * Complementa a `cedulaSchema`, no lo reemplaza: la regex sigue validando
 * al enviar, por si el valor llega de otro lado (`defaultValues`, pegar
 * texto de una fuente que no pasó por este `onChange`). */
export function sanearSoloDigitos(valor: string): string {
  return valor.replace(/\D/g, "");
}

/** Igual que `sanearSoloDigitos`, para el campo nombre. */
export function sanearSoloLetras(valor: string): string {
  return valor.replace(/[^\p{L}\s'-]/gu, "");
}
