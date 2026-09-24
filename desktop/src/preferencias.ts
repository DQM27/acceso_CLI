/**
 * Preferencias de interfaz por usuario en `localStorage` (menú lateral,
 * tema) -- pedido del usuario 2026-09-24: cada usuario del sistema tiene
 * las suyas en la misma PC. Mismo prefijo `u{id}:` que ya usan los layouts
 * de grilla (`idPorUsuario` en Tabla.tsx).
 *
 * Antes estas claves eran globales de la máquina. Si el usuario todavía no
 * guardó nada propio, se lee la clave global vieja como punto de partida,
 * así nadie pierde lo que ya tenía acomodado; el primer guardado ya queda
 * a su nombre.
 *
 * `localStorage` puede fallar (modo privado, cuota llena): leer devuelve
 * `null` y guardar no hace nada -- perder una preferencia no es motivo para
 * romper la pantalla.
 */

export function clavePorUsuario(clave: string, usuarioId: number | null): string {
  return usuarioId == null ? clave : `u${usuarioId}:${clave}`;
}

export function leerPreferencia(clave: string, usuarioId: number | null): string | null {
  try {
    return (
      localStorage.getItem(clavePorUsuario(clave, usuarioId)) ?? localStorage.getItem(clave)
    );
  } catch {
    return null;
  }
}

export function guardarPreferencia(clave: string, usuarioId: number | null, valor: string) {
  try {
    localStorage.setItem(clavePorUsuario(clave, usuarioId), valor);
  } catch {
    // Ver el doc-comment del módulo.
  }
}
