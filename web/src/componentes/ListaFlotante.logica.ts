import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

/**
 * Buscador con lista de resultados flotante — el mecanismo (no el contenido)
 * que `NuevoIngresoModal` y `SalidaModal` reimplementaban byte a byte cada
 * uno: posicionar la lista por coordenadas reales del campo, portal a
 * `document.body` para escapar el `overflow-y:auto` del modal, y navegación
 * con flechas/Enter. El contenido de cada fila (contratista vs. ingreso
 * activo) sigue siendo de cada pantalla — acá sólo vive lo que era idéntico.
 */

export interface PosicionLista {
  top: number;
  /** Distancia del borde superior del campo al borde inferior de la
   * ventana — para `direccion="arriba"` en `ListaFlotante` (disparadores
   * pegados al borde inferior, ej. `MenuUsuario` en la barra de estado, sin
   * espacio debajo para abrir para el lado de siempre). */
  bottom: number;
  left: number;
  /** Distancia del borde derecho del campo al borde derecho de la ventana —
   * para `alinear="derecha"` en `ListaFlotante` (ver ese componente). */
  right: number;
  width: number;
  /** Borde superior del campo y distancia de su borde IZQUIERDO al borde
   * derecho de la ventana (+4 de separación) -- para `direccion="izquierda"`
   * en `ListaFlotante`: el popover se abre al costado, a la misma altura. */
  topCampo: number;
  rightDesdeIzquierda: number;
}

/** Posición de la lista para el rectángulo del campo -- aparte para poder
 * testearla y compararla. */
export function posicionParaCampo(
  rect: Pick<DOMRect, "top" | "bottom" | "left" | "right" | "width">,
  ventana: { ancho: number; alto: number },
): PosicionLista {
  return {
    top: rect.bottom + 4,
    bottom: ventana.alto - rect.top + 4,
    left: rect.left,
    right: ventana.ancho - rect.right,
    width: rect.width,
    topCampo: rect.top,
    rightDesdeIzquierda: ventana.ancho - rect.left + 4,
  };
}

export function mismaPosicion(a: PosicionLista | null, b: PosicionLista): boolean {
  return (
    a !== null &&
    a.top === b.top &&
    a.bottom === b.bottom &&
    a.left === b.left &&
    a.right === b.right &&
    a.width === b.width
  );
}

/** Sigue la posición del campo (`campoRef`) mientras `visible` sea `true`.
 *
 * Se mide en cada cuadro (`requestAnimationFrame`) y sólo se actualiza si
 * cambió: el campo puede moverse sin que cambie la ventana -- p. ej. el
 * modal se estira o encoge animado al abrir o cerrar una ficha
 * (`.modal-cuerpo`). Antes se medía una sola vez al aparecer la lista, y si
 * se escribía mientras el modal se encogía, la lista quedaba desfasada del
 * buscador (reportado 2026-09-24). */
export function useListaFlotante(visible: boolean) {
  const campoRef = useRef<HTMLDivElement>(null);
  const [posicion, setPosicion] = useState<PosicionLista | null>(null);

  useLayoutEffect(() => {
    if (!visible || !campoRef.current) {
      setPosicion(null);
      return;
    }
    let cuadro = 0;
    const medir = () => {
      if (campoRef.current) {
        const nueva = posicionParaCampo(campoRef.current.getBoundingClientRect(), {
          ancho: window.innerWidth,
          alto: window.innerHeight,
        });
        setPosicion((actual) => (mismaPosicion(actual, nueva) ? actual : nueva));
      }
      cuadro = requestAnimationFrame(medir);
    };
    medir();
    return () => cancelAnimationFrame(cuadro);
  }, [visible]);

  return { campoRef, posicion };
}

/** ↑/↓ mueve `resaltado` dentro de `items`, Enter llama `onSeleccionar` con
 * el ítem resaltado. Se reinicia a 0 cada vez que cambia la lista de items
 * (si no, una búsqueda nueva con menos resultados puede dejarlo apuntando a
 * un índice que ya no existe). `activo` es la misma condición de
 * "¿la lista está visible?" que ya calcula cada pantalla (búsqueda con
 * texto, no en modo gafete, etc.) — el hook no la vuelve a inventar. */
export function useNavegacionFlechas<T>(
  items: T[],
  activo: boolean,
  onSeleccionar: (item: T) => void,
) {
  const [resaltado, setResaltado] = useState(0);

  useEffect(() => {
    // `Promise.resolve().then(...)` en vez de llamar `setResaltado` directo
    // -- ver el mismo comentario en Activos.tsx.
    Promise.resolve().then(() => setResaltado(0));
  }, [items]);

  function manejarTecla(evento: KeyboardEvent<HTMLInputElement>) {
    if (!activo || items.length === 0) return;
    if (evento.key === "ArrowDown") {
      evento.preventDefault();
      setResaltado((actual) => Math.min(actual + 1, items.length - 1));
    } else if (evento.key === "ArrowUp") {
      evento.preventDefault();
      setResaltado((actual) => Math.max(actual - 1, 0));
    } else if (evento.key === "Enter") {
      evento.preventDefault();
      onSeleccionar(items[resaltado]);
    }
  }

  return { resaltado, setResaltado, manejarTecla };
}
