import { useLayoutEffect, useState } from "react";
import { Moon, Sun } from "lucide-react";

const CLAVE_TEMA = "escritorio:tema";

type Tema = "light" | "dark";

export function temaDelSistema(): Tema {
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  } catch {
    return "light";
  }
}

export function leerTema(): Tema {
  try {
    const guardado = localStorage.getItem(CLAVE_TEMA);
    return guardado === "dark" || guardado === "light" ? guardado : temaDelSistema();
  } catch {
    return temaDelSistema();
  }
}

/**
 * Botón claro/oscuro en la barra de estado (junto a `MenuUsuario`) --
 * `diseno.css` ya soporta `[data-theme="light"|"dark"]` como override
 * explícito por encima de `prefers-color-scheme`; acá sólo faltaba el
 * control para tocarlo desde la UI y guardar la elección. Copiado de
 * `web/src/componentes/SelectorTema.tsx` (mismo diseno.css generado, mismas
 * clases).
 *
 * `useLayoutEffect` (no `useEffect`) para aplicar el atributo antes del
 * primer paint -- si no, cuando el tema guardado contradice el del sistema
 * operativo, se ve un parpadeo de un frame con el tema equivocado.
 */
export default function SelectorTema() {
  const [tema, setTema] = useState<Tema>(leerTema);

  useLayoutEffect(() => {
    document.documentElement.setAttribute("data-theme", tema);
  }, [tema]);

  function alternar() {
    setTema((actual) => {
      const siguiente: Tema = actual === "dark" ? "light" : "dark";
      try {
        localStorage.setItem(CLAVE_TEMA, siguiente);
      } catch {
        // localStorage puede fallar (modo privado, cuota llena) -- perder
        // la preferencia guardada no es motivo para romper el toggle.
      }
      return siguiente;
    });
  }

  const alModoClaro = tema === "dark";

  return (
    <button
      type="button"
      className="barra-estado-boton boton-icono"
      onClick={alternar}
      title={alModoClaro ? "Cambiar a modo claro" : "Cambiar a modo oscuro"}
      aria-label={alModoClaro ? "Cambiar a modo claro" : "Cambiar a modo oscuro"}
    >
      {alModoClaro ? (
        <Sun size={15} strokeWidth={2} aria-hidden="true" />
      ) : (
        <Moon size={15} strokeWidth={2} aria-hidden="true" />
      )}
    </button>
  );
}
