import { leerPreferencia } from "../preferencias";

/** Los dos temas de la app: Tokyo Day (claro) y Tokyo Night (oscuro).
 * Se generan desde `design/brisas.json` en `tokyo.css`. */
export type Tema = "tokyo-day" | "tokyo-night";

export function temaDelSistema(): Tema {
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "tokyo-night" : "tokyo-day";
  } catch {
    return "tokyo-day";
  }
}

export const CLAVE_TEMA = "escritorio:tema";

/** Último tema que se usó en esta PC, de cualquier usuario: es el que se ve
 * antes de iniciar sesión (ver `temaInicial`). Clave aparte de `CLAVE_TEMA`
 * porque `leerPreferencia` cae a la clave global si el usuario no guardó la
 * suya, y no debe heredar el tema de otra persona por eso. */
export const CLAVE_TEMA_ULTIMO = "escritorio:tema-ultimo";

/** Nombre con que se muestra cada tema en el menú de usuario. */
export const NOMBRE_TEMA: Record<Tema, string> = {
  // Claro/oscuro, no el nombre de la paleta: quien opera no tiene por qué
  // saber qué es "Tokyo Day" (auditoría 2026-11-14, U11).
  "tokyo-day": "Claro",
  "tokyo-night": "Oscuro",
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

/** Tema del login, el primer arranque y la pantalla de carga: el último que
 * se usó en esta PC o, si nunca se eligió ninguno, el del sistema. Antes ahí
 * se veía la paleta Brisas y al entrar cambiaba a Tokyo: un salto de color
 * en cada inicio de sesión (auditoría 2026-11-14, fase 2). */
export function temaInicial(): Tema {
  const guardado = leerPreferencia(CLAVE_TEMA_ULTIMO, null) ?? "";
  return TEMAS.includes(guardado as Tema) ? (guardado as Tema) : temaDelSistema();
}

export function siguienteTema(actual: Tema): Tema {
  return TEMAS[(TEMAS.indexOf(actual) + 1) % TEMAS.length];
}
