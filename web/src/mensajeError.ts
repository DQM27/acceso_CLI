/**
 * Formatea un error atrapado para mostrárselo al usuario -- sin esto,
 * `String(error)` sobre un `Error` real produce literalmente
 * "Error: &lt;mensaje&gt;" (el prefijo del `toString()` de `Error`), visible en
 * el toast/mensaje que ve quien opera el panel. Ya vivía duplicado (bien
 * en algunos lugares, mal -- `String(error)` directo -- en otros trece) en
 * cada pantalla; ahora es una sola función compartida.
 */
export function mensajeError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
