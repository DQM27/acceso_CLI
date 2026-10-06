// Reglas compartidas de las cuentas de anfitriones (`admin-anfitriones` y
// `anfitrion-activar`). Ver la migración
// `activacion_de_anfitriones_desde_el_panel` y
// docs/auditorias/investigacion-login-correo-web-visitas-2026-10-06.md.

/** Mismo alfabeto que las temporales de operadores (`admin-create-usuario`):
 * sin 0/O/1/I, se transcribe a mano. 32 símbolos (256 % 32 = 0, sin sesgo),
 * 10 caracteres = 50 bits; con 5 intentos y 72 h de vida es de sobra. */
const ALFABETO_CODIGO = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
export const LONGITUD_CODIGO = 10;

export function generarCodigo(): string {
  const bytes = new Uint8Array(LONGITUD_CODIGO);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => ALFABETO_CODIGO[b % ALFABETO_CODIGO.length]).join("");
}

/** Lo que escribió la persona, sin espacios ni guiones y en mayúsculas;
 * `null` si no puede ser un código. */
export function normalizarCodigo(valor: unknown): string | null {
  if (typeof valor !== "string") return null;
  const codigo = valor.replace(/[\s-]+/g, "").toUpperCase();
  return codigo.length === LONGITUD_CODIGO && [...codigo].every((c) => ALFABETO_CODIGO.includes(c)) ? codigo : null;
}

/** Contraseña de Auth para una cuenta pendiente o restablecida: nadie la
 * conoce, solo invalida la anterior. 256 bits; el sufijo fijo cubre
 * cualquier "password requirements" que se configure en Supabase. */
export function contrasenaAleatoria(): string {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  const base64 = btoa(String.fromCharCode(...bytes)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  return `${base64}aA1!`;
}

/** Correo como lo guarda Supabase Auth; `null` si no tiene forma de correo. */
export function normalizarCorreo(valor: unknown): string | null {
  if (typeof valor !== "string") return null;
  const correo = valor.trim().toLowerCase();
  return correo.length <= 254 && /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(correo) ? correo : null;
}

/** Dominio interno de los correos sintéticos de operadores: nunca anfitrión. */
export function esCorreoDeOperador(correo: string): boolean {
  return correo.endsWith("@brisas.local");
}

// Política de contraseñas (NIST SP 800-63B-4): la contraseña es el único
// factor, así que se exige longitud, no composición. Misma regla que
// web-visitas/src/lib/contrasena.ts; acá es la que manda, porque el mínimo
// global de Supabase no puede subir a 15 sin romper las temporales de 10 de
// los operadores.
export const LONGITUD_MINIMA_CONTRASENA = 15;
const BYTES_MAXIMOS_CONTRASENA = 72; // bcrypt

/** `null` si sirve; si no, el motivo para la persona. */
export function problemaDeContrasena(contrasena: string, correo: string): string | null {
  if ([...contrasena].length < LONGITUD_MINIMA_CONTRASENA) {
    return `Use al menos ${LONGITUD_MINIMA_CONTRASENA} caracteres.`;
  }
  if (new TextEncoder().encode(contrasena).length > BYTES_MAXIMOS_CONTRASENA) {
    return "La contraseña es demasiado larga.";
  }
  const plegada = contrasena.toLowerCase().replace(/\s+/g, "");
  if (new Set(plegada).size < 4) return "La contraseña tiene demasiados caracteres repetidos.";
  const usuario = correo.split("@")[0];
  if (usuario.length >= 4 && plegada.includes(usuario)) return "No use su correo dentro de la contraseña.";
  return null;
}
