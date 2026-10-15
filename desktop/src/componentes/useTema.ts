import { useLayoutEffect, useState } from "react";
import { guardarPreferencia } from "../preferencias";
import { CLAVE_TEMA, leerTema, siguienteTema } from "./SelectorTema.logica";
import type { Tema } from "./SelectorTema.logica";

/**
 * El tema de la app para este usuario: lo lee (o toma el del sistema), lo
 * aplica al documento y devuelve cómo cambiarlo. Se usa una sola vez, en el
 * `Shell`, para que el tema quede aplicado aunque ningún menú esté abierto;
 * quien lo muestra o lo cambia (`MenuUsuario`) lo recibe como dato.
 *
 * `useLayoutEffect` (no `useEffect`) para aplicar el atributo antes del
 * primer paint -- si no, cuando el tema guardado contradice el del sistema
 * operativo, se ve un parpadeo de un frame con el tema equivocado.
 */
export function useTema(usuarioId: number | null): { tema: Tema; alternarTema: () => void } {
  const [tema, setTema] = useState<Tema>(() => leerTema(usuarioId));

  useLayoutEffect(() => {
    document.documentElement.setAttribute("data-theme", tema);
  }, [tema]);

  function alternarTema() {
    setTema((actual) => {
      const siguiente = siguienteTema(actual);
      guardarPreferencia(CLAVE_TEMA, usuarioId, siguiente);
      return siguiente;
    });
  }

  return { tema, alternarTema };
}
