// Mismas reglas que desktop/src/validacion.ts -- sanitiza en vivo mientras
// se escribe (`onChange`), dejando sólo lo válido para cada campo. Copiado
// en vez de compartido porque desktop y web son paquetes npm separados sin
// workspace común todavía -- si en algún momento se suman más reglas
// compartidas, ahí sí vale la pena extraer un paquete propio.

/** Deja sólo dígitos -- para el campo cédula. */
export function sanearSoloDigitos(valor: string): string {
  return valor.replace(/\D/g, "");
}

/** Deja sólo letras (con acentos/ñ), espacios, apóstrofes y guiones -- para
 * el campo nombre. */
export function sanearSoloLetras(valor: string): string {
  return valor.replace(/[^\p{L}\s'-]/gu, "");
}
