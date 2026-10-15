import { useLayoutEffect, useState } from "react";
import { Moon, Sun } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useUsuarioId } from "../contexto/SesionContexto";
import { guardarPreferencia } from "../preferencias";
import { CLAVE_TEMA, leerTema, siguienteTema } from "./SelectorTema.logica";
import type { Tema } from "./SelectorTema.logica";

const NOMBRE_TEMA: Record<Tema, string> = {
  "tokyo-day": "Tokyo Night Day",
  "tokyo-night": "Tokyo Night",
};

/** Ícono del tema al que se pasa con el próximo clic. */
const ICONO_TEMA: Record<Tema, LucideIcon> = {
  "tokyo-day": Sun,
  "tokyo-night": Moon,
};

/**
 * Botón de tema en la barra de estado (junto a `MenuUsuario`): alterna entre
 * Tokyo Night Day y Tokyo Night (`tokyo-day.css` y `tokyo-night.css`, sobre
 * los tokens de `diseno.css`) y guarda la elección, por usuario.
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
