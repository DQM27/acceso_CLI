// Política de contraseñas de anfitriones (NIST SP 800-63B-4, 2025): la
// contraseña es el único factor, así que se exige longitud y no
// composición. Sin reglas de "mayúscula + número + símbolo", sin
// caducidad periódica y sin bloquear el pegado (los gestores de
// contraseñas son bienvenidos). La comprobación contra contraseñas
// filtradas y el mínimo autoritativo los hace Supabase Auth en el servidor;
// esto sólo evita un viaje inútil y explica el porqué al usuario.
// Ver docs/auditorias/seguridad-login-correo-web-visitas-2026-10-06.md.

/** Mínimo para una contraseña que es el único factor (NIST: 15). */
export const LONGITUD_MINIMA = 15;
/** bcrypt, que usa Supabase Auth, sólo mira los primeros 72 bytes; más
 * largo se rechaza en el servidor. */
export const BYTES_MAXIMOS = 72;

// Las más obvias en este contexto. La lista grande (HaveIBeenPwned) la
// aplica el servidor con "Prevent use of leaked passwords".
const PROHIBIDAS = ["contraseña", "contrasena", "password", "megabrisas", "brisas", "lattis", "visitas"];

/** Correo como lo guarda Supabase Auth (y como está en `anfitriones`). */
export function normalizarCorreo(correo: string): string {
  return correo.trim().toLowerCase();
}

/** `null` si la contraseña nueva es aceptable; si no, el motivo para el
 * usuario. */
export function problemaDeContrasenaNueva(contrasena: string, correo: string): string | null {
  if ([...contrasena].length < LONGITUD_MINIMA)
    return `Use al menos ${LONGITUD_MINIMA} caracteres. Una frase de varias palabras es fácil de recordar y difícil de adivinar.`;
  if (new TextEncoder().encode(contrasena).length > BYTES_MAXIMOS)
    return "Es demasiado larga. Use una frase un poco más corta.";
  const plegada = contrasena.toLowerCase().replace(/\s+/g, "");
  if (new Set(plegada).size < 4) return "Tiene demasiados caracteres repetidos.";
  const usuario = normalizarCorreo(correo).split("@")[0];
  if (usuario.length >= 4 && plegada.includes(usuario.replace(/\s+/g, "")))
    return "No use su correo dentro de la contraseña.";
  // "Contraseña2026!" o "megabrisas123456" son la palabra obvia con relleno.
  const soloLetras = plegada.replace(/[^\p{L}]+/gu, "");
  if (PROHIBIDAS.some((palabra) => soloLetras === palabra))
    return "Es demasiado fácil de adivinar. Use una frase propia.";
  return null;
}
