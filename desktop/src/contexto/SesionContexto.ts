import { createContext, useContext } from "react";

/** Quién tiene la sesión abierta, disponible para toda la Shell sin
 * prop-drilling por cada pantalla. `Tabla.tsx` usa el id para namespacear
 * layouts de grilla por usuario en localStorage (ver `claveAlmacenamiento`);
 * los reportes de gafetes muestran el nombre ("Queda a nombre de…"). `null`
 * fuera de una sesión (no debería pasar en la práctica: todo se monta dentro
 * de `Shell`, que sí provee un valor). */
export interface UsuarioEnSesion {
  id: number;
  nombre: string;
}

const SesionContexto = createContext<UsuarioEnSesion | null>(null);

export const SesionProvider = SesionContexto.Provider;

export function useUsuarioId(): number | null {
  return useContext(SesionContexto)?.id ?? null;
}

export function useNombreUsuario(): string | null {
  return useContext(SesionContexto)?.nombre ?? null;
}
