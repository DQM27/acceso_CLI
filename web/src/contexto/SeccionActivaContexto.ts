import { createContext, useContext } from "react";

/**
 * "¿Soy la sección visible ahora?". El panel deja montadas las secciones ya
 * visitadas (ocultas con `display: none`, ver `visitadas` en App.tsx), así
 * que una pantalla oculta sigue viva. Lo usan:
 *
 * - `useAutoRefresh`: oculta, no vuelve a pedir datos a Supabase; queda
 *   pendiente y refresca una vez al volver.
 * - `Tabla`: reaplica el layout guardado al volver a mostrarse (AG Grid
 *   recalcula las columnas `flex` cuando el contenedor vuelve a verse).
 *
 * Por defecto `true`: fuera de una sección (login, pruebas) todo está a la
 * vista. Mismo contexto que `SeccionActivaContexto` del escritorio.
 */
const SeccionActivaContexto = createContext(true);

export const SeccionActivaProvider = SeccionActivaContexto.Provider;

export function useSeccionActiva(): boolean {
  return useContext(SeccionActivaContexto);
}
