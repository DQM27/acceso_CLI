import { useLayoutEffect, useState } from "react";
import { Moon, Sparkles, Sun } from "lucide-react";
import type { LucideIcon } from "lucide-react";

const CLAVE_TEMA = "escritorio:tema";

export type Tema = "light" | "dark" | "tokyo-night";

/** Orden en que el botón recorre los temas (claro → oscuro → Tokyo Night
 * → claro). Tokyo Night se define en `tokyo-night.css`. */
const TEMAS: Tema[] = ["light", "dark", "tokyo-night"];

const NOMBRE_TEMA: Record<Tema, string> = {
  light: "modo claro",
  dark: "modo oscuro",
  "tokyo-night": "Tokyo Night",
};

/** Ícono del tema al que se pasa con el próximo clic. */
const ICONO_TEMA: Record<Tema, LucideIcon> = {
  light: Sun,
  dark: Moon,
  "tokyo-night": Sparkles,
};

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
    return TEMAS.includes(guardado as Tema) ? (guardado as Tema) : temaDelSistema();
  } catch {
    return temaDelSistema();
  }
}

export function siguienteTema(actual: Tema): Tema {
  return TEMAS[(TEMAS.indexOf(actual) + 1) % TEMAS.length];
}

/**
 * Botón de tema en la barra de estado (junto a `MenuUsuario`) --
 * `diseno.css` ya soporta `[data-theme="light"|"dark"]` como override
 * explícito por encima de `prefers-color-scheme`, y `tokyo-night.css` suma
 * `[data-theme="tokyo-night"]`; acá está el control para recorrerlos y
 * guardar la elección. El claro/oscuro se copió de
 * `web/src/componentes/SelectorTema.tsx` (mismo diseno.css generado, mismas
 * clases); Tokyo Night es sólo del escritorio.
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
      const siguiente = siguienteTema(actual);
      try {
        localStorage.setItem(CLAVE_TEMA, siguiente);
      } catch {
        // localStorage puede fallar (modo privado, cuota llena) -- perder
        // la preferencia guardada no es motivo para romper el toggle.
      }
      return siguiente;
    });
  }

  const proximo = siguienteTema(tema);
  const Icono = ICONO_TEMA[proximo];
  const etiqueta = `Cambiar a ${NOMBRE_TEMA[proximo]}`;

  return (
    <button
      type="button"
      className="barra-estado-boton boton-icono"
      onClick={alternar}
      title={etiqueta}
      aria-label={etiqueta}
    >
      <Icono size={15} strokeWidth={2} aria-hidden="true" />
    </button>
  );
}
