import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { createPortal } from "react-dom";

/**
 * Buscador con lista de resultados flotante — posiciona la lista por
 * coordenadas reales del campo, portal a `document.body` para escapar el
 * `overflow-y:auto` de un modal, y navegación con flechas/Enter. Copiado de
 * `desktop/src/componentes/ListaFlotante.tsx`.
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
}

/** Recalcula la posición del campo (`campoRef`) mientras `visible` sea
 * `true`, incluyendo al cambiar el tamaño de la ventana. */
export function useListaFlotante(visible: boolean) {
  const campoRef = useRef<HTMLDivElement>(null);
  const [posicion, setPosicion] = useState<PosicionLista | null>(null);

  useLayoutEffect(() => {
    if (!visible || !campoRef.current) {
      setPosicion(null);
      return;
    }
    const actualizar = () => {
      const rect = campoRef.current!.getBoundingClientRect();
      setPosicion({
        top: rect.bottom + 4,
        bottom: window.innerHeight - rect.top + 4,
        left: rect.left,
        right: window.innerWidth - rect.right,
        width: rect.width,
      });
    };
    actualizar();
    window.addEventListener("resize", actualizar);
    return () => window.removeEventListener("resize", actualizar);
  }, [visible]);

  return { campoRef, posicion };
}

/** ↑/↓ mueve `resaltado` dentro de `items`, Enter llama `onSeleccionar` con
 * el ítem resaltado. Se reinicia a 0 cada vez que cambia la lista de items
 * (si no, una búsqueda nueva con menos resultados puede dejarlo apuntando a
 * un índice que ya no existe). */
export function useNavegacionFlechas<T>(
  items: T[],
  activo: boolean,
  onSeleccionar: (item: T) => void,
) {
  const [resaltado, setResaltado] = useState(0);

  useEffect(() => {
    setResaltado(0);
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

/** El portal + tarjeta posicionada — flotante a propósito: los resultados no
 * deben empujar el resto del modal ni cambiar su tamaño, se superponen a lo
 * que haya debajo (como un autocompletar), y desaparecen solos al elegir o
 * al borrar el texto. */
export function ListaFlotante({
  posicion,
  ancho,
  alinear = "izquierda",
  direccion = "abajo",
  children,
}: {
  posicion: PosicionLista;
  /** Ancho fijo, en vez del ancho de `posicion` (el del campo/botón que
   * dispara la lista) — para contenido que no tiene por qué calzar con eso,
   * como el popover de `SelectorRangoFecha`. */
  ancho?: number;
  /** "derecha" ancla el borde derecho del popover al borde derecho del
   * campo/botón en vez del izquierdo — para disparadores pegados al borde
   * derecho de la pantalla. */
  alinear?: "izquierda" | "derecha";
  /** "arriba" ancla el popover por `bottom` (crece hacia arriba desde justo
   * encima del disparador) en vez de `top` — para disparadores pegados al
   * borde inferior de la ventana. */
  direccion?: "abajo" | "arriba";
  children: ReactNode;
}) {
  return createPortal(
    <div
      className="tarjeta"
      style={{
        position: "fixed",
        ...(direccion === "arriba" ? { bottom: posicion.bottom } : { top: posicion.top }),
        ...(alinear === "derecha" ? { right: posicion.right } : { left: posicion.left }),
        width: ancho ?? posicion.width,
        zIndex: 1000,
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        background: "var(--elevado)",
        boxShadow: "var(--sombra-panel)",
      }}
    >
      {children}
    </div>,
    document.body,
  );
}

/** Una fila de `ListaFlotante` — mismo estilo de resaltado (fondo + borde de
 * acento a la izquierda) en todos lados que la usen. */
export function FilaListaFlotante({
  resaltada,
  onClick,
  onMouseEnter,
  children,
}: {
  resaltada: boolean;
  onClick: () => void;
  onMouseEnter: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      onMouseEnter={onMouseEnter}
      style={{
        display: "flex",
        justifyContent: "space-between",
        gap: "0.75rem",
        padding: "0.6rem 0.8rem",
        border: "none",
        borderBottom: "1px solid var(--borde)",
        background: resaltada ? "var(--acento-suave)" : "var(--panel)",
        boxShadow: resaltada ? "inset 3px 0 0 var(--acento)" : "none",
        color: "var(--texto)",
        textAlign: "left",
        cursor: "pointer",
      }}
    >
      {children}
    </button>
  );
}

/** El "sin resultados" que muestran las listas cuando hay texto pero `items`
 * quedó vacío. */
export function SinResultados() {
  return (
    <p style={{ margin: 0, padding: "0.75rem", color: "var(--muted)" }}>Sin resultados.</p>
  );
}
