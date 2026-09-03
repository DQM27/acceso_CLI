import { createContext, useContext, useEffect } from "react";

/** Setter del mensaje visible en la barra de estado — la publica `Shell`
 * (App.tsx), que la renderiza de lado a lado en la parte inferior, debajo
 * de sidebar + contenido (mismo lugar que la barra de estado de VSC, no
 * "flotando" adentro de cada pantalla). Cada pantalla llama `useBarraEstado`
 * con su propio texto; no hay prop-drilling porque `Shell` no sabe de
 * antemano qué pantalla está montada. Valor por defecto no-op para que
 * llamar el hook fuera de `Shell` (ej. en un test que monta la pantalla
 * sola) no rompa nada. */
const BarraEstadoContexto = createContext<(mensaje: string | null) => void>(() => {});

export const BarraEstadoProvider = BarraEstadoContexto.Provider;

/** Publica `mensaje` en la barra de estado mientras el componente que llama
 * esto está montado — lo limpia (vuelve a `null`) al desmontar, así no
 * queda pegado el mensaje de la pantalla anterior al cambiar de sección. */
export function useBarraEstado(mensaje: string | null) {
  const establecer = useContext(BarraEstadoContexto);
  useEffect(() => {
    establecer(mensaje);
    return () => establecer(null);
  }, [mensaje, establecer]);
}
