import { useLayoutEffect, useState } from "react";

const CLAVE_TEMA = "web:tema";

/** Los dos temas del panel, los mismos de escritorio: Tokyo Day (claro) y
 * Tokyo Night (oscuro). Se definen en `tokyo-day.css` y `tokyo-night.css`. */
export type Tema = "tokyo-day" | "tokyo-night";

export const NOMBRE_TEMA: Record<Tema, string> = {
  "tokyo-day": "Tokyo Day",
  "tokyo-night": "Tokyo Night",
};

/** Lo que se guardaba antes de los temas Tokyo: "claro" y "oscuro" pasan a su
 * equivalente, así nadie pierde su elección. */
const TEMAS_ANTERIORES: Record<string, Tema> = { light: "tokyo-day", dark: "tokyo-night" };

function temaDelSistema(): Tema {
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "tokyo-night" : "tokyo-day";
  } catch {
    return "tokyo-day";
  }
}

export function leerTema(): Tema {
  try {
    const guardado = localStorage.getItem(CLAVE_TEMA) ?? "";
    if (guardado === "tokyo-day" || guardado === "tokyo-night") return guardado;
    return TEMAS_ANTERIORES[guardado] ?? temaDelSistema();
  } catch {
    return temaDelSistema();
  }
}

/**
 * Tema del panel: lo lee (o toma el del sistema), lo aplica al documento
 * (`data-theme`) y devuelve cómo cambiarlo. El control vive en el menú de la
 * cuenta (`MenuUsuario`), como en escritorio.
 *
 * `useLayoutEffect` (no `useEffect`) para aplicar el atributo antes del
 * primer paint -- si no, cuando el tema guardado contradice el del sistema
 * operativo, se ve un parpadeo de un frame con el tema equivocado.
 */
export function useTema(): { tema: Tema; alternarTema: () => void } {
  const [tema, setTema] = useState<Tema>(leerTema);

  useLayoutEffect(() => {
    document.documentElement.setAttribute("data-theme", tema);
  }, [tema]);

  function alternarTema() {
    setTema((actual) => {
      const siguiente: Tema = actual === "tokyo-night" ? "tokyo-day" : "tokyo-night";
      try {
        localStorage.setItem(CLAVE_TEMA, siguiente);
      } catch {
        // localStorage puede fallar (modo privado, cuota llena) -- perder
        // la preferencia guardada no es motivo para romper el cambio.
      }
      return siguiente;
    });
  }

  return { tema, alternarTema };
}
