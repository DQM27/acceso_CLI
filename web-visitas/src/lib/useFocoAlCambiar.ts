import { useEffect } from "react";
import type { RefObject } from "react";

/**
 * Mueve el foco de teclado a `referencia` cada vez que `dependencia` cambia
 * -- extraído del patrón que `NuevaCita.tsx` repetía a mano (foco al `<h1>`
 * al cambiar de paso, foco al resumen de errores cuando aparecen). El
 * elemento referenciado necesita `tabIndex={-1}` para poder recibir foco
 * programático sin quedar en el orden de tabulación normal.
 */
export function useFocoAlCambiar<T extends HTMLElement>(
  referencia: RefObject<T | null>,
  dependencia: unknown,
) {
  useEffect(() => {
    referencia.current?.focus();
    // Sólo debe re-disparar cuando cambia la dependencia -- `referencia` es
    // un ref, estable entre renders, no hace falta en el arreglo.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dependencia]);
}
