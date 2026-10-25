// Forma de los códigos de vinculación y de reconexión, sin dependencias ni
// variables de entorno: lo usan las Edge Functions y sus pruebas.

/**
 * Sin 0/O/1/I: se leen de una pantalla y a veces se escriben a mano. 32
 * símbolos dividen 256 exacto, así que `byte % 32` no sesga la elección.
 * 10 símbolos son 50 bits: imposible de adivinar en los minutos que vive.
 */
export const ALFABETO_CODIGO = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
export const LARGO_CODIGO = 10;

/** Mayúsculas y sin separadores; lo que no sea del alfabeto se descarta. */
export function normalizarCodigo(entrada: string): string {
  return entrada.toUpperCase().replace(/[^0-9A-Z]/g, "");
}

/** `K7QMR4XT2P` → `K7QM-R4XT-2P`, para mostrar. */
export function formatearCodigo(codigo: string): string {
  return [codigo.slice(0, 4), codigo.slice(4, 8), codigo.slice(8)].filter(Boolean).join("-");
}
