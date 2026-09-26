import { leerPreferencia } from "../preferencias";

export type Tema = "light" | "dark" | "tokyo-night";

export function temaDelSistema(): Tema {
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  } catch {
    return "light";
  }
}

export const CLAVE_TEMA = "escritorio:tema";

/** Orden en que el botón recorre los temas (claro → oscuro → Tokyo Night
 * → claro). Tokyo Night se define en `tokyo-night.css`. */
export const TEMAS: Tema[] = ["light", "dark", "tokyo-night"];

/** Tema guardado del usuario (cada usuario tiene el suyo, ver
 * `preferencias.ts`); sin guardado válido, el del sistema operativo. */
export function leerTema(usuarioId: number | null = null): Tema {
  const guardado = leerPreferencia(CLAVE_TEMA, usuarioId);
  return TEMAS.includes(guardado as Tema) ? (guardado as Tema) : temaDelSistema();
}

export function siguienteTema(actual: Tema): Tema {
  return TEMAS[(TEMAS.indexOf(actual) + 1) % TEMAS.length];
}
