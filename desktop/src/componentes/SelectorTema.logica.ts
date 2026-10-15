import { leerPreferencia } from "../preferencias";

/** Los dos temas de la app: Tokyo Night Day (claro) y Tokyo Night (oscuro).
 * Se definen en `tokyo-day.css` y `tokyo-night.css`. */
export type Tema = "tokyo-day" | "tokyo-night";

export function temaDelSistema(): Tema {
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "tokyo-night" : "tokyo-day";
  } catch {
    return "tokyo-day";
  }
}

export const CLAVE_TEMA = "escritorio:tema";

/** Nombre con que se muestra cada tema en el menú de usuario. */
export const NOMBRE_TEMA: Record<Tema, string> = {
  "tokyo-day": "Tokyo Night Day",
  "tokyo-night": "Tokyo Night",
};

/** Orden en que se recorren los temas (día → noche → día). */
export const TEMAS: Tema[] = ["tokyo-day", "tokyo-night"];

/** Lo que se guardaba antes de que los temas fueran estos dos: "claro" y
 * "oscuro" pasan a su equivalente Tokyo, así nadie pierde su elección. */
const TEMAS_ANTERIORES: Record<string, Tema> = {
  light: "tokyo-day",
  dark: "tokyo-night",
};

/** Tema guardado del usuario (cada usuario tiene el suyo, ver
 * `preferencias.ts`); sin guardado válido, el del sistema operativo. */
export function leerTema(usuarioId: number | null = null): Tema {
  const guardado = leerPreferencia(CLAVE_TEMA, usuarioId) ?? "";
  if (TEMAS.includes(guardado as Tema)) return guardado as Tema;
  return TEMAS_ANTERIORES[guardado] ?? temaDelSistema();
}

export function siguienteTema(actual: Tema): Tema {
  return TEMAS[(TEMAS.indexOf(actual) + 1) % TEMAS.length];
}
