import { createContext, useContext } from "react";

/** Id del usuario logueado, disponible para toda la Shell sin prop-drilling
 * por cada pantalla — hoy sólo lo usa `Tabla.tsx` para namespacear layouts
 * de grilla por usuario en localStorage (ver `claveAlmacenamiento`). `null`
 * fuera de una sesión (no debería pasar en la práctica: `Tabla` sólo se
 * monta dentro de `Shell`, que sí provee un valor). */
const SesionContexto = createContext<number | null>(null);

export const SesionProvider = SesionContexto.Provider;

export function useUsuarioId(): number | null {
  return useContext(SesionContexto);
}
