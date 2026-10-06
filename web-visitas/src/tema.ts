import { useLayoutEffect, useState } from "react";

const CLAVE_TEMA = "brisas:tema";
type Tema = "light" | "dark";

function temaInicial(): Tema {
  try {
    const guardado = localStorage.getItem(CLAVE_TEMA);
    if (guardado === "light" || guardado === "dark") return guardado;
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  } catch {
    return "light";
  }
}

/** Mismo mecanismo que el panel: `data-theme` en <html>, que diseno.css ya
 * entiende por encima de `prefers-color-scheme`. */
export function useTema() {
  const [tema, setTema] = useState<Tema>(temaInicial);
  useLayoutEffect(() => {
    document.documentElement.setAttribute("data-theme", tema);
  }, [tema]);
  const alternar = () => {
    const siguiente: Tema = tema === "light" ? "dark" : "light";
    setTema(siguiente);
    try {
      localStorage.setItem(CLAVE_TEMA, siguiente);
    } catch {
      /* La preferencia no es necesaria para usar la web. */
    }
  };
  return { tema, alternar };
}
