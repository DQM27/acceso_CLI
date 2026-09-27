import { createContext, useContext, useEffect, useRef } from "react";

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

/** "¿Soy la sección que se está mostrando ahora mismo?" -- `App.tsx` monta
 * TODAS las secciones ya visitadas a la vez (ocultas con CSS, no
 * desmontadas -- ver el doc-comment de `visitadas` en `Shell`), así que
 * `useBarraEstado` ya no puede confiar en "me desmontaron" para saber
 * cuándo dejar de estar activa. Cada sección queda envuelta en este
 * contexto con `true`/`false` según sea la visible o no. Default `true`
 * para que un test que monta una pantalla sola (sin `Shell` de por medio)
 * siga publicando su mensaje como siempre. */
const SeccionActivaContexto = createContext(true);

export const SeccionActivaProvider = SeccionActivaContexto.Provider;

/** "¿Soy la sección visible ahora?", para quien necesite reaccionar al
 * volver a activarse sin depender de `useBarraEstado` -- ver `Tabla.tsx`,
 * que reaplica el layout guardado cada vez que esto pasa a `true` (AG Grid
 * recalcula el ancho de columnas `flex` al remedir su contenedor, que pasa
 * de `display: none` a visible otra vez -- eso pisa el layout guardado, que
 * `onGridReady` sólo aplicó una vez, en el montaje inicial). */
export function useSeccionActiva() {
  return useContext(SeccionActivaContexto);
}

/** Publica `mensaje` en la barra de estado mientras el componente que llama
 * esto está MONTADO Y ACTIVO (ver `SeccionActivaContexto`) -- lo limpia
 * (vuelve a `null`) al desmontar o al dejar de ser la sección visible, así
 * no queda pegado el mensaje de la sección anterior al cambiar. Al volver
 * a activarse, se vuelve a publicar solo (el efecto corre de nuevo porque
 * `activa` es una dependencia) -- la pantalla nunca se desmontó, así que
 * su propio `mensaje` ya tiene el valor correcto esperando. */
export function useBarraEstado(mensaje: string | null) {
  const establecer = useContext(BarraEstadoContexto);
  const activa = useContext(SeccionActivaContexto);
  useEffect(() => {
    if (!activa) return;
    establecer(mensaje);
    return () => establecer(null);
  }, [mensaje, activa, establecer]);
}

/** Botón de acción en la barra de estado, junto al mensaje (ej. "Registrar
 * salida (2)" en Activos con filas seleccionadas). Vive ahí y no sobre la
 * grilla para que seleccionar filas no corra la pantalla -- pedido del
 * usuario 2026-09-23. */
export interface AccionBarraEstado {
  texto: string;
  alPulsar: () => void;
}

const AccionBarraEstadoContexto = createContext<(accion: AccionBarraEstado | null) => void>(
  () => {},
);

export const AccionBarraEstadoProvider = AccionBarraEstadoContexto.Provider;

/** Mismo ciclo de vida que `useBarraEstado`: publica la acción mientras la
 * sección esté activa, y la saca al desmontar o al cambiar de sección.
 * `texto` en `null` = sin botón. `alPulsar` se guarda en una ref, así el
 * efecto sólo depende del texto -- si dependiera de la función (nueva en
 * cada render), publicarla re-renderizaría `Shell` y éste a la pantalla,
 * en un ciclo sin fin. */
export function useAccionBarraEstado(texto: string | null, alPulsar: () => void) {
  const establecer = useContext(AccionBarraEstadoContexto);
  const activa = useContext(SeccionActivaContexto);
  const alPulsarRef = useRef(alPulsar);
  useEffect(() => {
    alPulsarRef.current = alPulsar;
  });
  useEffect(() => {
    if (!activa || texto === null) return;
    establecer({ texto, alPulsar: () => alPulsarRef.current() });
    return () => establecer(null);
  }, [texto, activa, establecer]);
}
