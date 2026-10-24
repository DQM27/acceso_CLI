import { createContext, useContext } from "react";
import type { UsuarioSesion } from "../api";

export interface EstadoAuth {
  sesion: UsuarioSesion | null;
  cargando: boolean;
  /** No confundir con "credenciales inválidas" — Google ya autenticó a esta
   * persona; esto significa que su correo no está en
   * `administradores_panel` (ver esa migración). */
  error: string | null;
  iniciarSesionConGoogle: () => Promise<void>;
  cerrarSesion: () => Promise<void>;
}

/** Lo provee `AuthProvider` (AuthContexto.tsx); vive aparte para que ese archivo sólo exporte componentes. */
export const AuthContexto = createContext<EstadoAuth | null>(null);

export function useAuth(): EstadoAuth {
  const contexto = useContext(AuthContexto);
  if (!contexto) throw new Error("useAuth debe usarse dentro de AuthProvider");
  return contexto;
}
