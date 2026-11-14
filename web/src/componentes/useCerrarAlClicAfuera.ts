import { useEffect, useEffectEvent } from "react";
import type { RefObject } from "react";

/**
 * Mientras `abierto`, llama `cerrar` al hacer clic fuera de todos los
 * elementos de `dentro` (el botón que abre y el menú/popover, que suele vivir
 * en un portal a `document.body` y por eso no es hijo del botón) y, con
 * `conEscape`, al apretar Escape.
 *
 * `mousedown` (no `click`) para cerrar antes de que un clic en otra parte del
 * panel dispare su propio manejador.
 *
 * Estaba copiado en MenuContextualSecciones, MenuUsuario, SelectorRangoFecha,
 * SelectorUnidadesOperativas y Tabla (auditoría 2026-11-14). Mismo hook que
 * `desktop/src/componentes/useCerrarAlClicAfuera.ts`.
 */
export function useCerrarAlClicAfuera(
  abierto: boolean,
  dentro: RefObject<HTMLElement | null>[],
  cerrar: () => void,
  { conEscape = true }: { conEscape?: boolean } = {},
) {
  // Siempre con los refs y el `cerrar` más recientes, sin volver a instalar
  // las escuchas en cada render.
  const alHacerClic = useEffectEvent((evento: MouseEvent) => {
    const objetivo = evento.target as Node;
    if (dentro.some((ref) => ref.current?.contains(objetivo))) return;
    cerrar();
  });
  const alTeclear = useEffectEvent((evento: KeyboardEvent) => {
    if (evento.key === "Escape") cerrar();
  });

  useEffect(() => {
    if (!abierto) return;
    const clic = (evento: MouseEvent) => alHacerClic(evento);
    const tecla = (evento: KeyboardEvent) => alTeclear(evento);
    document.addEventListener("mousedown", clic);
    if (conEscape) document.addEventListener("keydown", tecla);
    return () => {
      document.removeEventListener("mousedown", clic);
      document.removeEventListener("keydown", tecla);
    };
  }, [abierto, conEscape]);
}
