import { useLayoutEffect, useState } from "react";

const CLAVE_TEMA = "web:tema";

export type Tema = "light" | "dark";

export const NOMBRE_TEMA: Record<Tema, string> = { light: "Claro", dark: "Oscuro" };

function temaDelSistema(): Tema {
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  } catch {
    return "light";
  }
}

function leerTema(): Tema {
  try {
    const guardado = localStorage.getItem(CLAVE_TEMA);
    return guardado === "dark" || guardado === "light" ? guardado : temaDelSistema();
  } catch {
    return temaDelSistema();
  }
}

/**
 * Tema claro/oscuro del panel: `diseno.css` ya soporta
 * `[data-theme="light"|"dark"]` como override explícito por encima de
 * `prefers-color-scheme`; esto lo aplica y guarda la elección. El control
 * vive en el menú de la cuenta (`MenuUsuario`), como en escritorio.
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
      const siguiente: Tema = actual === "dark" ? "light" : "dark";
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
