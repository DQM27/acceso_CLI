/**
 * Código de vinculación que el panel genera para cada equipo (ver
 * `supabase/functions/_shared/dispositivos.ts`): 10 caracteres de un
 * alfabeto sin 0/O/1/I, que el panel muestra como `K7QM-R4XT-2P`. Acá sólo
 * se ayuda a escribirlo: la validación real la hace el servidor.
 */

export const LARGO_CODIGO = 10;

/** Mayúsculas y sin separadores ni espacios, como lo compara el servidor. */
export function normalizarCodigo(entrada: string): string {
  return entrada.toUpperCase().replace(/[^0-9A-Z]/g, "");
}

/**
 * Lo que se muestra en el campo mientras se escribe: agrupado en 4-4-2
 * como en el panel, y cortado al largo del código.
 */
export function formatearCodigo(entrada: string): string {
  const codigo = normalizarCodigo(entrada).slice(0, LARGO_CODIGO);
  return [codigo.slice(0, 4), codigo.slice(4, 8), codigo.slice(8)].filter(Boolean).join("-");
}

export function codigoCompleto(entrada: string): boolean {
  return normalizarCodigo(entrada).length === LARGO_CODIGO;
}
