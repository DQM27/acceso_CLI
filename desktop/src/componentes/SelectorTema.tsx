import { useLayoutEffect, useState } from "react";
import { Moon, Sparkles, Sun } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useUsuarioId } from "../contexto/SesionContexto";
import { guardarPreferencia, leerPreferencia } from "../preferencias";

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

/** Tema guardado del usuario (cada usuario tiene el suyo, ver
 * `preferencias.ts`); sin guardado válido, el del sistema operativo. */
export function leerTema(usuarioId: number | null = null): Tema {
  const guardado = leerPreferencia(CLAVE_TEMA, usuarioId);
  return TEMAS.includes(guardado as Tema) ? (guardado as Tema) : temaDelSistema();
}

export function siguienteTema(actual: Tema): Tema {
  return TEMAS[(TEMAS.indexOf(actual) + 1) % TEMAS.length];
}

/**
 * Botón de tema en la barra de estado (junto a `MenuUsuario`) --
 * `diseno.css` ya soporta `[data-theme="light"|"dark"]` como override
 * explícito por encima de `prefers-color-scheme`, y `tokyo-night.css` suma
 * `[data-theme="tokyo-night"]`; acá está el control para recorrerlos y
 * guardar la elección, por usuario. El claro/oscuro se copió de
 * `web/src/componentes/SelectorTema.tsx` (mismo diseno.css generado, mismas
 * clases); Tokyo Night es sólo del escritorio.
 *
 * `useLayoutEffect` (no `useEffect`) para aplicar el atributo antes del
 * primer paint -- si no, cuando el tema guardado contradice el del sistema
 * operativo, se ve un parpadeo de un frame con el tema equivocado.
 */
export default function SelectorTema() {
  const usuarioId = useUsuarioId();
  const [tema, setTema] = useState<Tema>(() => leerTema(usuarioId));

  useLayoutEffect(() => {
    document.documentElement.setAttribute("data-theme", tema);
  }, [tema]);

  function alternar() {
    setTema((actual) => {
      const siguiente = siguienteTema(actual);
      guardarPreferencia(CLAVE_TEMA, usuarioId, siguiente);
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
