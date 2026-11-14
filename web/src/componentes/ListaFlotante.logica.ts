import { useLayoutEffect, useRef, useState } from "react";

/**
 * Posición de una lista o menú flotante junto a su botón o campo: por
 * coordenadas reales, en un portal a `document.body` para escapar el
 * `overflow` de quien lo contiene (menú de usuario, selectores de fecha y de
 * unidades, "Columnas"). Copiado del escritorio; la navegación con flechas de
 * allá no se trajo porque el panel no tiene buscadores con lista (se quitó
 * la copia sin uso, auditoría 2026-11-14).
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
  /** Borde derecho del campo (+8 de separación) y distancia de su borde
   * INFERIOR al borde inferior de la ventana -- para `direccion="derecha"`
   * en `ListaFlotante`: el popover se abre al costado derecho, con su borde
   * inferior a la misma altura (disparadores al fondo de un menú lateral). */
  leftDesdeDerecha: number;
  bottomCampo: number;
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
    leftDesdeDerecha: rect.right + 8,
    bottomCampo: ventana.alto - rect.bottom,
  };
}

export function mismaPosicion(a: PosicionLista | null, b: PosicionLista): boolean {
  return (
    a !== null &&
    a.top === b.top &&
    a.bottom === b.bottom &&
    a.left === b.left &&
    a.right === b.right &&
    a.bottomCampo === b.bottomCampo &&
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
